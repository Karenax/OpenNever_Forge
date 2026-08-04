use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_resource::{ResourceCatalog, ResourceManager};
use std::sync::atomic::AtomicBool;

const PLT_HEADER_SIZE: usize = 24;
const MAX_TEXTURE_DIMENSION: u32 = 16_384;
const MAX_TEXTURE_PIXELS: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPreview {
    pub bytes: Vec<u8>,
    pub mime_type: &'static str,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub fn build_asset_preview(
    catalog: &ResourceCatalog,
    resref: &str,
    resource_type: u16,
    cancelled: &AtomicBool,
) -> AppResult<AssetPreview> {
    let key = ResourceKey::new(resref, resource_type);
    let resource = catalog.get(&key).ok_or_else(|| {
        Box::new(
            AppError::new(
                "ASSET_RESOURCE_NOT_FOUND",
                "La ressource demandée n'est pas disponible.",
                format!("Resource Manager did not resolve {key}"),
                ErrorSeverity::Warning,
            )
            .with_resource(key.to_string()),
        )
    })?;
    let bytes = ResourceManager::read(&resource.selected, cancelled)?;
    match resource_type {
        3 => direct(bytes, "image/x-tga"),
        6 => preview_plt(&bytes, &key),
        2033 => direct(bytes, "image/vnd-ms.dds"),
        2073 => direct(bytes, "image/ktx"),
        2079 => direct(bytes, "image/gif"),
        2080 => direct(bytes, "image/png"),
        2081 => direct(bytes, "image/jpeg"),
        _ => Err(Box::new(
            AppError::new(
                "ASSET_PREVIEW_TYPE_UNSUPPORTED",
                "Cette ressource ne possède pas d'aperçu binaire direct.",
                format!("resource type {resource_type} is not an image preview type"),
                ErrorSeverity::Warning,
            )
            .with_resource(key.to_string()),
        )),
    }
}

fn direct(bytes: Vec<u8>, mime_type: &'static str) -> AppResult<AssetPreview> {
    Ok(AssetPreview {
        bytes,
        mime_type,
        width: None,
        height: None,
    })
}

fn preview_plt(bytes: &[u8], key: &ResourceKey) -> AppResult<AssetPreview> {
    if bytes.len() < PLT_HEADER_SIZE || &bytes[..8] != b"PLT V1  " {
        return Err(plt_error(
            key,
            "PLT_HEADER_INVALID",
            "PLT signature is absent or truncated",
        ));
    }
    let width = u32::from_le_bytes(bytes[16..20].try_into().expect("fixed PLT width"));
    let height = u32::from_le_bytes(bytes[20..24].try_into().expect("fixed PLT height"));
    if width == 0 || height == 0 || width > MAX_TEXTURE_DIMENSION || height > MAX_TEXTURE_DIMENSION
    {
        return Err(plt_error(
            key,
            "PLT_DIMENSIONS_INVALID",
            format!("PLT dimensions {width}x{height} exceed defensive limits"),
        ));
    }
    let pixels = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .filter(|pixels| *pixels <= MAX_TEXTURE_PIXELS)
        .ok_or_else(|| {
            plt_error(
                key,
                "PLT_PIXEL_LIMIT_EXCEEDED",
                format!("PLT dimensions {width}x{height} exceed the pixel budget"),
            )
        })?;
    let payload_size = pixels
        .checked_mul(2)
        .ok_or_else(|| plt_error(key, "PLT_SIZE_OVERFLOW", "PLT pixel payload size overflow"))?;
    let payload = bytes
        .get(PLT_HEADER_SIZE..PLT_HEADER_SIZE + payload_size)
        .ok_or_else(|| {
            plt_error(
                key,
                "PLT_PAYLOAD_TRUNCATED",
                format!("PLT requires {payload_size} pixel bytes"),
            )
        })?;
    let mut rgba = Vec::with_capacity(pixels * 4);
    for pixel in payload.chunks_exact(2) {
        rgba.extend(layer_color(pixel[0], pixel[1]));
    }
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| plt_error(key, "PLT_PNG_HEADER_FAILED", error.to_string()))?;
        writer
            .write_image_data(&rgba)
            .map_err(|error| plt_error(key, "PLT_PNG_ENCODING_FAILED", error.to_string()))?;
    }
    Ok(AssetPreview {
        bytes: encoded,
        mime_type: "image/png",
        width: Some(width),
        height: Some(height),
    })
}

fn layer_color(value: u8, layer: u8) -> [u8; 4] {
    let luminance = u16::from(value);
    let tint = match layer {
        0 => [255_u16, 214, 181],
        1 => [86, 52, 36],
        2 => [190, 205, 220],
        3 => [217, 180, 72],
        4 => [199, 58, 62],
        5 => [52, 143, 214],
        6 => [136, 83, 45],
        7 => [79, 124, 63],
        8 => [155, 73, 183],
        9 => [45, 174, 164],
        _ => [255, 0, 255],
    };
    [
        ((tint[0] * luminance) / 255) as u8,
        ((tint[1] * luminance) / 255) as u8,
        ((tint[2] * luminance) / 255) as u8,
        255,
    ]
}

fn plt_error(
    key: &ResourceKey,
    code: impl Into<String>,
    technical_message: impl Into<String>,
) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "La texture PLT ne peut pas être prévisualisée.",
            technical_message,
            ErrorSeverity::Warning,
        )
        .with_resource(key.to_string())
        .with_import_stage("plt_preview"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_layer_preserving_plt_preview_as_png() {
        let mut bytes = b"PLT V1  \0\0\0\0\0\0\0\0".to_vec();
        bytes.extend(2_u32.to_le_bytes());
        bytes.extend(1_u32.to_le_bytes());
        bytes.extend([255, 0, 128, 4]);
        let preview = preview_plt(&bytes, &ResourceKey::new("body", 6)).expect("preview");
        assert_eq!(&preview.bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(preview.width, Some(2));
        assert_eq!(preview.height, Some(1));
        assert_eq!(preview.mime_type, "image/png");
    }

    #[test]
    fn rejects_truncated_plt_without_allocating_the_claimed_image() {
        let mut bytes = b"PLT V1  \0\0\0\0\0\0\0\0".to_vec();
        bytes.extend(16_384_u32.to_le_bytes());
        bytes.extend(16_384_u32.to_le_bytes());
        let error = preview_plt(&bytes, &ResourceKey::new("huge", 6)).expect_err("limit");
        assert_eq!(error.code, "PLT_PIXEL_LIMIT_EXCEEDED");
    }
}
