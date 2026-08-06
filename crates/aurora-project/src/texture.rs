use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_resource::{ResourceCatalog, ResourceManager};
use std::sync::atomic::AtomicBool;

const PLT_HEADER_SIZE: usize = 24;
const BIOWARE_DDS_HEADER_SIZE: usize = 20;
const STANDARD_DDS_HEADER_SIZE: usize = 128;
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
        2033 => preview_dds(&bytes, &key),
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

fn preview_dds(bytes: &[u8], key: &ResourceKey) -> AppResult<AssetPreview> {
    if bytes.starts_with(b"DDS ") {
        if bytes.len() < STANDARD_DDS_HEADER_SIZE {
            return Err(dds_error(
                key,
                "DDS_STANDARD_HEADER_TRUNCATED",
                "standard DDS header is truncated",
            ));
        }
        return Ok(AssetPreview {
            bytes: bytes.to_vec(),
            mime_type: "image/vnd-ms.dds",
            width: Some(u32::from_le_bytes(
                bytes[16..20].try_into().expect("DDS width"),
            )),
            height: Some(u32::from_le_bytes(
                bytes[12..16].try_into().expect("DDS height"),
            )),
        });
    }
    if bytes.len() < BIOWARE_DDS_HEADER_SIZE {
        return Err(dds_error(
            key,
            "DDS_BIOWARE_HEADER_TRUNCATED",
            "BioWare DDS header is absent or truncated",
        ));
    }

    let width = u32::from_le_bytes(bytes[0..4].try_into().expect("BioWare DDS width"));
    let height = u32::from_le_bytes(bytes[4..8].try_into().expect("BioWare DDS height"));
    if width == 0
        || height == 0
        || width > MAX_TEXTURE_DIMENSION
        || height > MAX_TEXTURE_DIMENSION
        || !width.is_power_of_two()
        || !height.is_power_of_two()
    {
        return Err(dds_error(
            key,
            "DDS_BIOWARE_DIMENSIONS_INVALID",
            format!("BioWare DDS dimensions {width}x{height} are invalid"),
        ));
    }

    let encoding = u32::from_le_bytes(bytes[8..12].try_into().expect("BioWare DDS encoding"));
    let block_bytes = match encoding {
        3 => 8_usize,
        4 => 16_usize,
        _ => {
            return Err(dds_error(
                key,
                "DDS_BIOWARE_ENCODING_UNSUPPORTED",
                format!("BioWare DDS encoding {encoding} is unsupported"),
            ));
        }
    };
    let base_size = compressed_mip_size(width, height, block_bytes).ok_or_else(|| {
        dds_error(
            key,
            "DDS_BIOWARE_SIZE_OVERFLOW",
            "BioWare DDS base mip size overflow",
        )
    })?;
    let declared_size = usize::try_from(u32::from_le_bytes(
        bytes[12..16]
            .try_into()
            .expect("BioWare DDS base data size"),
    ))
    .expect("u32 always fits usize on supported desktop targets");
    if declared_size != base_size {
        return Err(dds_error(
            key,
            "DDS_BIOWARE_BASE_SIZE_INVALID",
            format!("BioWare DDS declares {declared_size} base bytes, expected {base_size}"),
        ));
    }

    let payload = &bytes[BIOWARE_DDS_HEADER_SIZE..];
    let mut mip_width = width;
    let mut mip_height = height;
    let mut consumed = 0_usize;
    let mut mip_count = 0_u32;
    loop {
        let mip_size =
            compressed_mip_size(mip_width, mip_height, block_bytes).ok_or_else(|| {
                dds_error(
                    key,
                    "DDS_BIOWARE_SIZE_OVERFLOW",
                    "BioWare DDS mip size overflow",
                )
            })?;
        if payload.len().saturating_sub(consumed) < mip_size {
            break;
        }
        consumed += mip_size;
        mip_count += 1;
        if mip_width == 1 && mip_height == 1 {
            break;
        }
        mip_width = (mip_width / 2).max(1);
        mip_height = (mip_height / 2).max(1);
    }
    if mip_count == 0 || consumed != payload.len() {
        return Err(dds_error(
            key,
            "DDS_BIOWARE_PAYLOAD_INVALID",
            format!(
                "BioWare DDS payload has {} bytes; {consumed} form complete mip levels",
                payload.len()
            ),
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
            dds_error(
                key,
                "DDS_BIOWARE_PIXEL_LIMIT_EXCEEDED",
                format!("BioWare DDS dimensions {width}x{height} exceed the preview pixel budget"),
            )
        })?;
    let rgba = decode_bioware_dds_base_mip(width, height, encoding, &payload[..base_size], pixels);
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| {
            dds_error(
                key,
                "DDS_BIOWARE_PNG_HEADER_FAILED",
                format!("DDS preview PNG header failed: {error}"),
            )
        })?;
        writer.write_image_data(&rgba).map_err(|error| {
            dds_error(
                key,
                "DDS_BIOWARE_PNG_WRITE_FAILED",
                format!("DDS preview PNG write failed: {error}"),
            )
        })?;
    }

    Ok(AssetPreview {
        bytes: encoded,
        mime_type: "image/png",
        width: Some(width),
        height: Some(height),
    })
}

fn compressed_mip_size(width: u32, height: u32, block_bytes: usize) -> Option<usize> {
    let blocks_wide = usize::try_from(width.div_ceil(4)).ok()?.max(1);
    let blocks_high = usize::try_from(height.div_ceil(4)).ok()?.max(1);
    blocks_wide
        .checked_mul(blocks_high)?
        .checked_mul(block_bytes)
}

fn decode_bioware_dds_base_mip(
    width: u32,
    height: u32,
    encoding: u32,
    payload: &[u8],
    pixels: usize,
) -> Vec<u8> {
    let block_bytes = if encoding == 3 { 8 } else { 16 };
    let blocks_wide = width.div_ceil(4) as usize;
    let blocks_high = height.div_ceil(4) as usize;
    let mut rgba = vec![0_u8; pixels * 4];
    for block_y in 0..blocks_high {
        for block_x in 0..blocks_wide {
            let offset = (block_y * blocks_wide + block_x) * block_bytes;
            let block = &payload[offset..offset + block_bytes];
            let (colors, color_bits) = if encoding == 3 {
                (
                    decode_bc_colors(block, false),
                    u32::from_le_bytes(block[4..8].try_into().expect("BC1 indices")),
                )
            } else {
                (
                    decode_bc_colors(&block[8..16], true),
                    u32::from_le_bytes(block[12..16].try_into().expect("BC3 color indices")),
                )
            };
            let alpha_values = if encoding == 4 {
                Some(decode_bc3_alphas(block))
            } else {
                None
            };
            let alpha_bits = if encoding == 4 {
                let mut bytes = [0_u8; 8];
                bytes[..6].copy_from_slice(&block[2..8]);
                u64::from_le_bytes(bytes)
            } else {
                0
            };
            for local_y in 0..4_usize {
                for local_x in 0..4_usize {
                    let x = block_x * 4 + local_x;
                    let y = block_y * 4 + local_y;
                    if x >= width as usize || y >= height as usize {
                        continue;
                    }
                    let pixel_in_block = local_y * 4 + local_x;
                    let color_index = ((color_bits >> (pixel_in_block * 2)) & 0x3) as usize;
                    let mut color = colors[color_index];
                    if let Some(alphas) = alpha_values {
                        let alpha_index = ((alpha_bits >> (pixel_in_block * 3)) & 0x7) as usize;
                        color[3] = alphas[alpha_index];
                    }
                    let target = (y * width as usize + x) * 4;
                    rgba[target..target + 4].copy_from_slice(&color);
                }
            }
        }
    }
    rgba
}

fn decode_bc_colors(block: &[u8], force_four_colors: bool) -> [[u8; 4]; 4] {
    let color_0_raw = u16::from_le_bytes(block[0..2].try_into().expect("BC color 0"));
    let color_1_raw = u16::from_le_bytes(block[2..4].try_into().expect("BC color 1"));
    let color_0 = rgb565(color_0_raw);
    let color_1 = rgb565(color_1_raw);
    let mut colors = [color_0, color_1, [0, 0, 0, 255], [0, 0, 0, 255]];
    if force_four_colors || color_0_raw > color_1_raw {
        for channel in 0..3 {
            colors[2][channel] =
                ((2 * u16::from(color_0[channel]) + u16::from(color_1[channel])) / 3) as u8;
            colors[3][channel] =
                ((u16::from(color_0[channel]) + 2 * u16::from(color_1[channel])) / 3) as u8;
        }
    } else {
        for channel in 0..3 {
            colors[2][channel] =
                ((u16::from(color_0[channel]) + u16::from(color_1[channel])) / 2) as u8;
        }
        colors[3] = [0, 0, 0, 0];
    }
    colors
}

fn rgb565(value: u16) -> [u8; 4] {
    let red = ((u32::from((value >> 11) & 0x1f) * 255 + 15) / 31) as u8;
    let green = ((u32::from((value >> 5) & 0x3f) * 255 + 31) / 63) as u8;
    let blue = ((u32::from(value & 0x1f) * 255 + 15) / 31) as u8;
    [red, green, blue, 255]
}

fn decode_bc3_alphas(block: &[u8]) -> [u8; 8] {
    let alpha_0 = block[0];
    let alpha_1 = block[1];
    let mut alphas = [alpha_0, alpha_1, 0, 0, 0, 0, 0, 255];
    if alpha_0 > alpha_1 {
        for index in 1..=6_u16 {
            alphas[(index + 1) as usize] =
                (((7 - index) * u16::from(alpha_0) + index * u16::from(alpha_1)) / 7) as u8;
        }
    } else {
        for index in 1..=4_u16 {
            alphas[(index + 1) as usize] =
                (((5 - index) * u16::from(alpha_0) + index * u16::from(alpha_1)) / 5) as u8;
        }
        alphas[6] = 0;
    }
    alphas
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

fn dds_error(
    key: &ResourceKey,
    code: impl Into<String>,
    technical_message: impl Into<String>,
) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "La texture DDS ne peut pas être prévisualisée.",
            technical_message,
            ErrorSeverity::Warning,
        )
        .with_resource(key.to_string())
        .with_import_stage("dds_preview"),
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

    #[test]
    fn decodes_bioware_dxt1_to_png() {
        let mut source = Vec::new();
        source.extend(4_u32.to_le_bytes());
        source.extend(4_u32.to_le_bytes());
        source.extend(3_u32.to_le_bytes());
        source.extend(8_u32.to_le_bytes());
        source.extend(1_f32.to_le_bytes());
        source.extend([0_u8; 8]);

        let preview = preview_dds(&source, &ResourceKey::new("stone", 2033)).expect("preview");
        assert_eq!(&preview.bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(preview.mime_type, "image/png");
        assert_eq!((preview.width, preview.height), (Some(4), Some(4)));
    }

    #[test]
    fn decodes_bioware_dxt5_with_complete_mip_levels() {
        let mut source = Vec::new();
        source.extend(8_u32.to_le_bytes());
        source.extend(8_u32.to_le_bytes());
        source.extend(4_u32.to_le_bytes());
        source.extend(64_u32.to_le_bytes());
        source.extend(0.5_f32.to_le_bytes());
        source.extend([0_u8; 64 + 16 + 16 + 16]);

        let preview = preview_dds(&source, &ResourceKey::new("leaves", 2033)).expect("preview");
        assert_eq!(&preview.bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(preview.mime_type, "image/png");
        assert_eq!((preview.width, preview.height), (Some(8), Some(8)));
    }

    #[test]
    fn decodes_bc1_color_indices() {
        let block = [0x00, 0xf8, 0xe0, 0x07, 0, 0, 0, 0];
        let rgba = decode_bioware_dds_base_mip(4, 4, 3, &block, 16);
        assert_eq!(&rgba[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn rejects_incomplete_bioware_dds_mip_payload() {
        let mut source = Vec::new();
        source.extend(4_u32.to_le_bytes());
        source.extend(4_u32.to_le_bytes());
        source.extend(3_u32.to_le_bytes());
        source.extend(8_u32.to_le_bytes());
        source.extend(1_f32.to_le_bytes());
        source.extend([0_u8; 7]);

        let error =
            preview_dds(&source, &ResourceKey::new("broken", 2033)).expect_err("truncated payload");
        assert_eq!(error.code, "DDS_BIOWARE_PAYLOAD_INVALID");
    }
}
