# Application desktop

Frontend React/TypeScript et point d'entrée Tauri d'OpenNever Forge.

Les règles métier, les lecteurs et SQLite restent dans les crates Rust du workspace. Le frontend
utilise uniquement les fonctions de `src/lib/tauri.ts` pour communiquer avec le cœur.
