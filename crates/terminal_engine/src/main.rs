use std::{
    env, fs,
    path::{Path, PathBuf},
};
use terminal_engine::{TerminalReelGame, run_terminal_reel};

const GAME_SYMBOL: &[u8] = b"terminal_game_definition_v1\0";

fn main() {
    if let Err(error) = run() {
        eprintln!("terminal-engine: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let context = LaunchContext::from_environment()?;
    print_loaded(&context);

    let game_library = context.game_library()?;
    println!("Game Artifact Library");
    println!("  artifact: {}", context.game_id);
    println!("  library: {}", game_library.display());
    println!();

    let game = load_game_definition(&game_library)?;
    run_terminal_reel(game)
}

#[derive(Debug)]
struct LaunchContext {
    target: String,
    packagepack_id: String,
    packagepack_root: PathBuf,
    engine_id: String,
    engine_root: PathBuf,
    runtime_target: String,
    game_id: String,
    game_root: PathBuf,
    game_title: String,
    game_library_name: String,
}

impl LaunchContext {
    fn from_environment() -> Result<Self, String> {
        let target = required_env("VAPOR_LAUNCH_TARGET")?;
        let packagepack_id = required_env("VAPOR_PACKAGEPACK_ID")?;
        let packagepack_root = PathBuf::from(required_env("VAPOR_PACKAGEPACK_ROOT")?);
        let engine_id = required_env("VAPOR_ENGINE_ID")?;
        let engine_root = PathBuf::from(required_env("VAPOR_ENGINE_ROOT")?);
        let runtime_target = required_env("VAPOR_RUNTIME_TARGET")?;

        let packagepack_manifest = read_manifest(&packagepack_root, "Packagepack.vapor.toml")?;
        let game_id = manifest_dependency_id(&packagepack_manifest, "packagepack", "game")
            .or_else(|| manifest_value(&packagepack_manifest, "packagepack.game", "id"))
            .ok_or_else(|| {
                format!(
                    "packagepack manifest has no game dependency: {}",
                    packagepack_root.join("Packagepack.vapor.toml").display()
                )
            })?;
        let installed_root = installed_content_root(&packagepack_root, &packagepack_id)?;
        let game_root = installed_root.join(id_path(&game_id));
        if !game_root.is_dir() {
            return Err(format!(
                "packagepack game is not installed: {game_id}\n  expected: {}",
                game_root.display()
            ));
        }

        let game_manifest = read_manifest(&game_root, "Game.vapor.toml")?;
        let game_engine_id = manifest_dependency_id(&game_manifest, "game", "engine")
            .or_else(|| manifest_value(&game_manifest, "game.engine", "id"))
            .ok_or_else(|| {
                format!(
                    "game manifest has no engine dependency: {}",
                    game_root.join("Game.vapor.toml").display()
                )
            })?;
        if game_engine_id != engine_id {
            return Err(format!(
                "game '{}' requires engine '{}', but packagepack selected '{}'",
                game_id, game_engine_id, engine_id
            ));
        }

        let game_title = manifest_value(&game_manifest, "game.steam", "title")
            .or_else(|| manifest_value(&game_manifest, "game", "name"))
            .unwrap_or_else(|| short_id(&game_id).to_owned());
        let game_library_name = manifest_array_strings(&game_manifest, "game", "libraries")
            .and_then(|items| items.into_iter().next())
            .ok_or_else(|| {
                format!(
                    "game manifest declares no runtime library: {}",
                    game_root.join("Game.vapor.toml").display()
                )
            })?;

        Ok(Self {
            target,
            packagepack_id,
            packagepack_root,
            engine_id,
            engine_root,
            runtime_target,
            game_id,
            game_root,
            game_title,
            game_library_name,
        })
    }

    fn game_library(&self) -> Result<PathBuf, String> {
        let path = self
            .game_root
            .join("lib")
            .join(&self.runtime_target)
            .join(library_file_name(
                &self.game_library_name,
                &self.runtime_target,
            ));
        if path.is_file() {
            Ok(path)
        } else {
            Err(format!(
                "installed game library is missing: {}\nhelp: declare and deploy the game library from the game artifact",
                path.display()
            ))
        }
    }
}

fn print_loaded(context: &LaunchContext) {
    println!();
    println!("Terminal Engine Example");
    println!();
    println!("Loaded Packagepack Composition");
    println!("  launch target: {}", context.target);
    println!("  packagepack artifact: {}", context.packagepack_id);
    println!("    root: {}", context.packagepack_root.display());
    println!("  game artifact: {}", context.game_id);
    println!("    display title: {}", context.game_title);
    println!("    root: {}", context.game_root.display());
    println!("  engine artifact: {}", context.engine_id);
    println!("    root: {}", context.engine_root.display());
    println!("  runtime target: {}", context.runtime_target);
    println!();
}

#[cfg(target_os = "linux")]
fn load_game_definition(library: &Path) -> Result<TerminalReelGame, String> {
    linux_loader::load_game_definition(library, GAME_SYMBOL)
}

#[cfg(windows)]
fn load_game_definition(library: &Path) -> Result<TerminalReelGame, String> {
    windows_loader::load_game_definition(library, GAME_SYMBOL)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn load_game_definition(_library: &Path) -> Result<TerminalReelGame, String> {
    Err(
        "the dynamic terminal example currently implements runtime loading on Linux and Windows"
            .to_owned(),
    )
}

#[cfg(target_os = "linux")]
mod linux_loader {
    use super::TerminalReelGame;
    use std::{ffi, os::unix::ffi::OsStrExt, path::Path};

    type GameDefinition = unsafe extern "Rust" fn() -> TerminalReelGame;

    pub fn load_game_definition(library: &Path, symbol: &[u8]) -> Result<TerminalReelGame, String> {
        let library = DynamicLibrary::open(library)?;
        let definition: GameDefinition = unsafe { library.symbol(symbol)? };
        let game = unsafe { definition() };
        std::mem::forget(library);
        Ok(game)
    }

    struct DynamicLibrary {
        handle: *mut ffi::c_void,
    }

    impl DynamicLibrary {
        fn open(path: &Path) -> Result<Self, String> {
            let path = ffi::CString::new(path.as_os_str().as_bytes())
                .map_err(|_| format!("library path contains NUL byte: {}", path.display()))?;
            let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW) };
            if handle.is_null() {
                Err(format!("failed to load game library: {}", dl_error()))
            } else {
                Ok(Self { handle })
            }
        }

        unsafe fn symbol<T>(&self, symbol: &[u8]) -> Result<T, String>
        where
            T: Copy,
        {
            let pointer = unsafe { dlsym(self.handle, symbol.as_ptr().cast()) };
            if pointer.is_null() {
                Err(format!(
                    "game library is missing symbol '{}': {}",
                    String::from_utf8_lossy(&symbol[..symbol.len().saturating_sub(1)]),
                    dl_error()
                ))
            } else {
                Ok(unsafe { std::mem::transmute_copy(&pointer) })
            }
        }
    }

    impl Drop for DynamicLibrary {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                unsafe {
                    dlclose(self.handle);
                }
            }
        }
    }

    const RTLD_NOW: ffi::c_int = 2;

    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(filename: *const ffi::c_char, flags: ffi::c_int) -> *mut ffi::c_void;
        fn dlsym(handle: *mut ffi::c_void, symbol: *const ffi::c_char) -> *mut ffi::c_void;
        fn dlclose(handle: *mut ffi::c_void) -> ffi::c_int;
        fn dlerror() -> *const ffi::c_char;
    }

    fn dl_error() -> String {
        let error = unsafe { dlerror() };
        if error.is_null() {
            "unknown dynamic loader error".to_owned()
        } else {
            unsafe { ffi::CStr::from_ptr(error) }
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[cfg(windows)]
mod windows_loader {
    use super::TerminalReelGame;
    use std::{ffi, os::windows::ffi::OsStrExt, path::Path};

    type GameDefinition = unsafe extern "Rust" fn() -> TerminalReelGame;
    type HModule = *mut ffi::c_void;

    pub fn load_game_definition(library: &Path, symbol: &[u8]) -> Result<TerminalReelGame, String> {
        let library = DynamicLibrary::open(library)?;
        let definition: GameDefinition = unsafe { library.symbol(symbol)? };
        let game = unsafe { definition() };
        std::mem::forget(library);
        Ok(game)
    }

    struct DynamicLibrary {
        handle: HModule,
    }

    impl DynamicLibrary {
        fn open(path: &Path) -> Result<Self, String> {
            let wide_path = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>();
            let handle = unsafe { LoadLibraryW(wide_path.as_ptr()) };
            if handle.is_null() {
                Err(format!(
                    "failed to load game library '{}': Windows error {}",
                    path.display(),
                    last_error()
                ))
            } else {
                Ok(Self { handle })
            }
        }

        unsafe fn symbol<T>(&self, symbol: &[u8]) -> Result<T, String>
        where
            T: Copy,
        {
            let pointer = unsafe { GetProcAddress(self.handle, symbol.as_ptr().cast()) };
            if pointer.is_null() {
                Err(format!(
                    "game library is missing symbol '{}': Windows error {}",
                    String::from_utf8_lossy(&symbol[..symbol.len().saturating_sub(1)]),
                    last_error()
                ))
            } else {
                Ok(unsafe { std::mem::transmute_copy(&pointer) })
            }
        }
    }

    impl Drop for DynamicLibrary {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                unsafe {
                    FreeLibrary(self.handle);
                }
            }
        }
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(lp_lib_file_name: *const u16) -> HModule;
        fn GetProcAddress(h_module: HModule, lp_proc_name: *const ffi::c_char) -> *mut ffi::c_void;
        fn FreeLibrary(h_lib_module: HModule) -> i32;
        fn GetLastError() -> u32;
    }

    fn last_error() -> u32 {
        unsafe { GetLastError() }
    }
}

fn required_env(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| {
        format!("missing {name}; launch this runtime through Vapor packagepack handoff")
    })
}

fn read_manifest(root: &Path, file_name: &str) -> Result<String, String> {
    let path = root.join(file_name);
    fs::read_to_string(&path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))
}

fn manifest_value(source: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = table_name(line) == Some(section);
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((name, value)) = line.split_once('=')
            && name.trim() == key
        {
            return unquote(value.trim()).map(ToOwned::to_owned);
        }
    }
    None
}

fn manifest_array_strings(source: &str, section: &str, key: &str) -> Option<Vec<String>> {
    let value = manifest_raw_value(source, section, key)?;
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let mut items = Vec::new();
    for item in inner.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        items.push(unquote(item)?.to_owned());
    }
    Some(items)
}

fn manifest_raw_value<'a>(source: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut in_section = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = table_name(line) == Some(section);
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((name, value)) = line.split_once('=')
            && name.trim() == key
        {
            return Some(value.trim());
        }
    }
    None
}

fn manifest_dependency_id(source: &str, parent: &str, relationship: &str) -> Option<String> {
    let dependencies = format!("{parent}.dependencies");
    let mut in_dependency = false;
    let mut id = None;
    let mut found_relationship = None;

    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if in_dependency && found_relationship.as_deref() == Some(relationship) {
                return id;
            }
            in_dependency = table_name(line) == Some(dependencies.as_str());
            id = None;
            found_relationship = None;
            continue;
        }
        if !in_dependency {
            continue;
        }
        if let Some((name, value)) = line.split_once('=') {
            match name.trim() {
                "id" => id = unquote(value.trim()).map(ToOwned::to_owned),
                "relationship" => found_relationship = unquote(value.trim()).map(ToOwned::to_owned),
                _ => {}
            }
        }
    }

    (in_dependency && found_relationship.as_deref() == Some(relationship))
        .then_some(id)
        .flatten()
}

fn table_name(line: &str) -> Option<&str> {
    line.strip_prefix("[[")
        .and_then(|value| value.strip_suffix("]]"))
        .or_else(|| {
            line.strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
        })
        .map(str::trim)
}

fn unquote(value: &str) -> Option<&str> {
    value.strip_prefix('"')?.strip_suffix('"')
}

fn installed_content_root(
    packagepack_root: &Path,
    packagepack_id: &str,
) -> Result<PathBuf, String> {
    let mut root = packagepack_root.to_path_buf();
    for _ in packagepack_id.split('/').filter(|part| !part.is_empty()) {
        if !root.pop() {
            return Err(format!(
                "cannot resolve installed content root from packagepack root '{}'",
                packagepack_root.display()
            ));
        }
    }
    Ok(root)
}

fn id_path(id: &str) -> PathBuf {
    id.split('/').filter(|part| !part.is_empty()).collect()
}

fn short_id(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}

fn library_file_name(stem: &str, target: &str) -> String {
    if Path::new(stem).extension().is_some() {
        stem.to_owned()
    } else if target.contains("darwin") || target.contains("apple") {
        format!("lib{stem}.dylib")
    } else if target.contains("windows") {
        format!("{stem}.dll")
    } else {
        format!("lib{}.so", stem.replace('-', "_"))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        installed_content_root, library_file_name, manifest_array_strings, manifest_dependency_id,
    };
    use std::path::Path;

    #[test]
    fn reads_deployed_packagepack_game_dependency() {
        let source = r#"
[[packagepack.dependencies]]
id = "ghf-studios/vapor-examples/terminal-engine"
relationship = "engine"

[[packagepack.dependencies]]
id = "ghf-studios/vapor-examples/hello-world-on-steroids-game"
relationship = "game"
"#;

        assert_eq!(
            manifest_dependency_id(source, "packagepack", "game").as_deref(),
            Some("ghf-studios/vapor-examples/hello-world-on-steroids-game")
        );
    }

    #[test]
    fn reads_game_library_from_deployed_manifest() {
        let source = r#"
[game]
libraries = ["hello-world-on-steroids-game"]
"#;

        assert_eq!(
            manifest_array_strings(source, "game", "libraries").unwrap(),
            vec!["hello-world-on-steroids-game"]
        );
    }

    #[test]
    fn resolves_installed_content_root_from_packagepack_path() {
        let root = installed_content_root(
            Path::new(
                "/app/content/installed/ghf-studios/vapor-examples/hello-world-on-steroids-packagepack",
            ),
            "ghf-studios/vapor-examples/hello-world-on-steroids-packagepack",
        )
        .unwrap();

        assert_eq!(root, Path::new("/app/content/installed"));
    }

    #[test]
    fn resolves_linux_library_name() {
        if cfg!(target_os = "linux") {
            assert_eq!(
                library_file_name("hello-world-on-steroids-game", "x86_64-unknown-linux-gnu"),
                "libhello_world_on_steroids_game.so"
            );
        }
    }
}
