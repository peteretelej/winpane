use std::collections::HashMap;
use std::path::PathBuf;

fn positions_file() -> Option<PathBuf> {
    let dir = dirs::data_local_dir()?.join("winpane");
    Some(dir.join("positions.json"))
}

#[cfg(target_os = "windows")]
pub fn load_position(key: &str) -> Option<(i32, i32)> {
    let path = positions_file()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let map: HashMap<String, serde_json::Value> = serde_json::from_str(&content).ok()?;
    let entry = map.get(key)?;
    let x = entry.get("x")?.as_i64()? as i32;
    let y = entry.get("y")?.as_i64()? as i32;
    Some((x, y))
}

#[cfg(target_os = "windows")]
pub fn save_position(key: &str, x: i32, y: i32) {
    let Some(path) = positions_file() else {
        return;
    };
    // Read existing, merge, write
    let mut map: HashMap<String, serde_json::Value> = path
        .exists()
        .then(|| std::fs::read_to_string(&path).ok())
        .flatten()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();
    map.insert(key.to_string(), serde_json::json!({ "x": x, "y": y }));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&map).unwrap_or_default(),
    );
}

#[cfg(target_os = "windows")]
pub fn is_position_on_screen(x: i32, y: i32, monitors: &[crate::types::MonitorInfo]) -> bool {
    monitors
        .iter()
        .any(|m| x >= m.x && x < m.x + m.width as i32 && y >= m.y && y < m.y + m.height as i32)
}

#[cfg(not(target_os = "windows"))]
pub fn load_position(_key: &str) -> Option<(i32, i32)> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn save_position(_key: &str, _x: i32, _y: i32) {}

#[cfg(not(target_os = "windows"))]
pub fn is_position_on_screen(_x: i32, _y: i32, _monitors: &[crate::types::MonitorInfo]) -> bool {
    false
}
