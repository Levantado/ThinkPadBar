use iced::widget::image::Handle;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IconLookupKey {
    raw: String,
    theme_hint: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct IconSearchEnv {
    home_dir: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    xdg_data_dirs: Vec<PathBuf>,
}

impl IconSearchEnv {
    fn from_process() -> Self {
        let home_dir = dirs::home_dir();
        let xdg_data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir.as_ref().map(|home| home.join(".local/share")));

        let xdg_data_dirs = std::env::var_os("XDG_DATA_DIRS")
            .map(|raw| std::env::split_paths(&raw).collect::<Vec<_>>())
            .filter(|paths| !paths.is_empty())
            .unwrap_or_else(|| {
                vec![
                    PathBuf::from("/usr/local/share"),
                    PathBuf::from("/usr/share"),
                ]
            });

        Self {
            home_dir,
            xdg_data_home,
            xdg_data_dirs,
        }
    }
}

#[derive(Default)]
pub struct IconResolver {
    cache: HashMap<IconLookupKey, Option<Handle>>,
}

impl IconResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resolve(&mut self, raw: &str, theme_hint: Option<&str>) -> Option<Handle> {
        let key = IconLookupKey {
            raw: raw.to_string(),
            theme_hint: theme_hint.map(str::to_string),
        };
        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }

        let env = IconSearchEnv::from_process();
        let resolved = resolve_icon_path(raw, theme_hint, &env)
            .map(|path| Handle::from_path(path.to_string_lossy().into_owned()));
        self.cache.insert(key, resolved.clone());
        resolved
    }
}

fn resolve_icon_path(raw: &str, theme_hint: Option<&str>, env: &IconSearchEnv) -> Option<PathBuf> {
    for candidate in icon_name_candidates(raw) {
        if let Some(path) = resolve_candidate_path(&candidate, theme_hint, env) {
            return Some(path);
        }
    }
    None
}

fn resolve_candidate_path(
    candidate: &str,
    theme_hint: Option<&str>,
    env: &IconSearchEnv,
) -> Option<PathBuf> {
    let direct = candidate.strip_prefix("file://").unwrap_or(candidate);
    let direct_path = Path::new(direct);
    if direct_path.exists() {
        return Some(direct_path.to_path_buf());
    }

    for theme_root in theme_search_roots(theme_hint, env) {
        for path in themed_icon_paths(&theme_root, candidate) {
            if path.exists() {
                return Some(path);
            }
        }
    }

    pixmap_paths(candidate, env)
        .into_iter()
        .find(|path| path.exists())
}

fn icon_name_candidates(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return out;
    }

    out.push(trimmed.to_string());

    let no_prefix = trimmed
        .strip_prefix("file://")
        .unwrap_or(trimmed)
        .trim_matches('"');
    if no_prefix != trimmed {
        out.push(no_prefix.to_string());
    }

    let file_name = Path::new(no_prefix)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(no_prefix);
    if !file_name.is_empty() && file_name != no_prefix {
        out.push(file_name.to_string());
    }

    let base = file_name
        .strip_suffix(".svg")
        .or_else(|| file_name.strip_suffix(".png"))
        .or_else(|| file_name.strip_suffix(".xpm"))
        .unwrap_or(file_name);
    if !base.is_empty() && base != file_name {
        out.push(base.to_string());
    }

    let mut stripped = base;
    if let Some(value) = stripped.strip_suffix("-symbolic") {
        if !value.is_empty() {
            out.push(value.to_string());
        }
        stripped = value;
    }
    if let Some(value) = stripped.strip_suffix("-panel") {
        if !value.is_empty() {
            out.push(value.to_string());
        }
    } else if let Some(value) = base.strip_suffix("-panel") {
        if !value.is_empty() {
            out.push(value.to_string());
        }
    }

    out.sort();
    out.dedup();
    out
}

fn theme_search_roots(theme_hint: Option<&str>, env: &IconSearchEnv) -> Vec<PathBuf> {
    let theme_dirs = icon_theme_dirs(env);
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();

    if let Some(theme_hint) = theme_hint.map(str::trim).filter(|hint| !hint.is_empty()) {
        if Path::new(theme_hint).is_absolute() {
            queue.push_back(PathBuf::from(theme_hint));
        } else {
            for root in theme_roots_for_name(theme_hint, &theme_dirs) {
                queue.push_back(root);
            }
        }
    }

    for root in theme_roots_for_name("hicolor", &theme_dirs) {
        queue.push_back(root);
    }

    while let Some(root) = queue.pop_front() {
        if !seen.insert(root.clone()) {
            continue;
        }

        if root.exists() {
            out.push(root.clone());
        }

        for inherited in inherited_theme_roots(&root, &theme_dirs) {
            if !seen.contains(&inherited) {
                queue.push_back(inherited);
            }
        }
    }

    out
}

fn icon_theme_dirs(env: &IconSearchEnv) -> Vec<PathBuf> {
    let mut dirs_out = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |path: PathBuf| {
        if seen.insert(path.clone()) {
            dirs_out.push(path);
        }
    };

    if let Some(home) = &env.home_dir {
        push(home.join(".icons"));
        push(home.join(".local/share/icons"));
        push(home.join(".local/share/flatpak/exports/share/icons"));
    }

    if let Some(data_home) = &env.xdg_data_home {
        push(data_home.join("icons"));
    }

    for data_dir in &env.xdg_data_dirs {
        push(data_dir.join("icons"));
    }

    push(PathBuf::from("/var/lib/flatpak/exports/share/icons"));
    dirs_out
}

fn theme_roots_for_name(name: &str, theme_dirs: &[PathBuf]) -> Vec<PathBuf> {
    theme_dirs.iter().map(|root| root.join(name)).collect()
}

fn inherited_theme_roots(root: &Path, theme_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let inherits = parse_theme_inherits(root);
    if inherits.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let parent_dir = root.parent().map(Path::to_path_buf);

    for inherit in inherits {
        if let Some(parent) = &parent_dir {
            let candidate = parent.join(&inherit);
            if seen.insert(candidate.clone()) {
                out.push(candidate);
            }
        }

        for candidate in theme_roots_for_name(&inherit, theme_dirs) {
            if seen.insert(candidate.clone()) {
                out.push(candidate);
            }
        }
    }

    out
}

fn parse_theme_inherits(root: &Path) -> Vec<String> {
    let index_path = root.join("index.theme");
    let Ok(contents) = std::fs::read_to_string(index_path) else {
        return Vec::new();
    };

    contents
        .lines()
        .find_map(|line| line.trim().strip_prefix("Inherits="))
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn themed_icon_paths(theme_root: &Path, name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let dirs = [
        "scalable/apps",
        "scalable/status",
        "scalable/panel",
        "symbolic/apps",
        "symbolic/status",
        "symbolic/panel",
        "16x16/apps",
        "16x16/status",
        "16x16/panel",
        "22x22/apps",
        "22x22/status",
        "22x22/panel",
        "24x24/apps",
        "24x24/status",
        "24x24/panel",
        "32x32/apps",
        "32x32/status",
        "32x32/panel",
        "48x48/apps",
        "48x48/status",
        "48x48/panel",
        "64x64/apps",
        "64x64/status",
        "64x64/panel",
        "128x128/apps",
        "128x128/status",
        "128x128/panel",
        "256x256/apps",
        "256x256/status",
        "256x256/panel",
    ];
    let exts = ["svg", "png", "xpm"];

    for ext in exts {
        paths.push(theme_root.join(format!("{name}.{ext}")));
        for dir in dirs {
            paths.push(theme_root.join(dir).join(format!("{name}.{ext}")));
        }
    }
    paths
}

fn pixmap_paths(name: &str, env: &IconSearchEnv) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |path: PathBuf| {
        if seen.insert(path.clone()) {
            out.push(path);
        }
    };

    let base_dirs = if let Some(data_home) = &env.xdg_data_home {
        let mut dirs = vec![data_home.clone()];
        dirs.extend(env.xdg_data_dirs.iter().cloned());
        dirs
    } else {
        env.xdg_data_dirs.clone()
    };

    for base in base_dirs {
        push(base.join("pixmaps").join(format!("{name}.png")));
        push(base.join("pixmaps").join(format!("{name}.svg")));
        push(base.join("pixmaps").join(format!("{name}.xpm")));
    }

    if let Some(home) = &env.home_dir {
        push(
            home.join(".local/share/pixmaps")
                .join(format!("{name}.png")),
        );
        push(
            home.join(".local/share/pixmaps")
                .join(format!("{name}.svg")),
        );
        push(
            home.join(".local/share/pixmaps")
                .join(format!("{name}.xpm")),
        );
    }

    push(PathBuf::from("/usr/share/pixmaps").join(format!("{name}.png")));
    push(PathBuf::from("/usr/share/pixmaps").join(format!("{name}.svg")));
    push(PathBuf::from("/usr/share/pixmaps").join(format!("{name}.xpm")));
    out
}

#[cfg(test)]
fn resolve_icon_path_for_tests(
    raw: &str,
    theme_hint: Option<&str>,
    env: &IconSearchEnv,
) -> Option<PathBuf> {
    resolve_icon_path(raw, theme_hint, env)
}

#[cfg(test)]
mod tests {
    use super::{
        icon_name_candidates, resolve_icon_path_for_tests, theme_search_roots, themed_icon_paths,
        IconResolver, IconSearchEnv,
    };
    use std::path::{Path, PathBuf};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be monotonic enough for tests")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[test]
    fn icon_name_candidates_strip_common_tray_suffixes() {
        let c = icon_name_candidates("sample-panel-symbolic");
        assert!(c.iter().any(|v| v == "sample-panel-symbolic"));
        assert!(c.iter().any(|v| v == "sample-panel"));
        assert!(c.iter().any(|v| v == "sample"));
    }

    #[test]
    fn themed_icon_paths_cover_panel_locations() {
        let p = themed_icon_paths(Path::new("/usr/share/icons/Papirus-Dark"), "sample-panel");
        assert!(p
            .iter()
            .any(|v| v
                == &PathBuf::from("/usr/share/icons/Papirus-Dark/22x22/panel/sample-panel.svg")));
        assert!(p
            .iter()
            .any(|v| v
                == &PathBuf::from("/usr/share/icons/Papirus-Dark/24x24/status/sample-panel.png")));
    }

    #[test]
    fn named_theme_search_roots_include_xdg_and_legacy_locations() {
        let temp = unique_temp_dir("thinkpadbar-icon-search-roots");
        let home = temp.join("home");
        let data_home = home.join(".local/share");
        let icons_home = home.join(".icons/Papirus-Dark");
        let icons_data_home = data_home.join("icons/Papirus-Dark");
        let usr_share = temp.join("usr/share/icons/Papirus-Dark");
        std::fs::create_dir_all(&icons_home).expect("home icons dir");
        std::fs::create_dir_all(&icons_data_home).expect("data home icons dir");
        std::fs::create_dir_all(&usr_share).expect("usr share icons dir");

        let env = IconSearchEnv {
            home_dir: Some(home),
            xdg_data_home: Some(data_home),
            xdg_data_dirs: vec![temp.join("usr/local/share"), temp.join("usr/share")],
        };

        let roots = theme_search_roots(Some("Papirus-Dark"), &env);
        assert!(roots.iter().any(|path| path == &icons_home));
        assert!(roots.iter().any(|path| path == &icons_data_home));
        assert!(roots.iter().any(|path| path == &usr_share));

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn resolver_supports_inherited_theme_lookup_from_absolute_root() {
        let temp = unique_temp_dir("thinkpadbar-icon-theme");
        let themes_root = temp.join("icons");
        let custom = themes_root.join("CustomTheme");
        let hicolor = themes_root.join("hicolor");
        std::fs::create_dir_all(custom.join("24x24/status")).expect("custom dir");
        std::fs::create_dir_all(hicolor.join("24x24/status")).expect("hicolor dir");
        std::fs::write(
            custom.join("index.theme"),
            "[Icon Theme]\nName=CustomTheme\nInherits=hicolor\n",
        )
        .expect("index.theme");
        std::fs::write(hicolor.join("24x24/status/sample-panel.png"), b"png").expect("icon");

        let env = IconSearchEnv {
            home_dir: None,
            xdg_data_home: None,
            xdg_data_dirs: Vec::new(),
        };
        let resolved = resolve_icon_path_for_tests(
            "sample-panel",
            Some(custom.to_string_lossy().as_ref()),
            &env,
        )
        .expect("icon should resolve via inherited theme");
        assert_eq!(resolved, hicolor.join("24x24/status/sample-panel.png"));

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn resolver_negative_cache_is_stable() {
        let mut resolver = IconResolver::new();
        assert!(resolver.resolve("definitely-missing-icon", None).is_none());
        assert!(resolver.resolve("definitely-missing-icon", None).is_none());
    }
}
