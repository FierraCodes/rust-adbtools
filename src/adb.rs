use std::collections::HashSet;
use std::process::{Command, Stdio};

#[derive(Clone, Debug)]
pub struct Device {
    pub serial: String,
    pub state: String,
    pub model: String,
}

impl Device {
    pub fn label(&self) -> String {
        if self.model.is_empty() {
            format!("{} ({})", self.serial, self.state)
        } else {
            format!("{} [{}] ({})", self.model, self.serial, self.state)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub apk: String,
    pub system: bool,
    pub disabled: bool,
}

/// Run `adb [-s serial] args...` and return stdout (or stderr text on failure).
pub fn run(serial: Option<&str>, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("adb");
    if let Some(s) = serial {
        cmd.args(["-s", s]);
    }
    let out = cmd
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("failed to run adb: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() && !stdout.starts_with("Failure") {
        Ok(if stdout.is_empty() { stderr } else { stdout })
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

pub fn shell(serial: &str, cmd: &str) -> Result<String, String> {
    run(Some(serial), &["shell", cmd])
}

pub fn devices() -> Result<Vec<Device>, String> {
    let out = run(None, &["devices", "-l"])?;
    Ok(out
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty() && !l.starts_with('*'))
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let serial = it.next()?.to_string();
            let state = it.next()?.to_string();
            let model = it
                .find_map(|t| t.strip_prefix("model:"))
                .unwrap_or("")
                .replace('_', " ");
            Some(Device { serial, state, model })
        })
        .collect())
}

fn pkg_set(serial: &str, flag: &str) -> HashSet<String> {
    shell(serial, &format!("pm list packages {flag}"))
        .map(|o| {
            o.lines()
                .filter_map(|l| l.trim().strip_prefix("package:"))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub fn packages(serial: &str) -> Result<Vec<Package>, String> {
    let all = shell(serial, "pm list packages -f -u")?;
    let user = pkg_set(serial, "-3");
    let disabled = pkg_set(serial, "-d");
    let mut v: Vec<Package> = all
        .lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .filter_map(|l| {
            // format: /path/to/base.apk=com.example (path may contain '=')
            let (apk, name) = l.rsplit_once('=')?;
            Some(Package {
                name: name.to_string(),
                apk: apk.to_string(),
                system: !user.contains(name),
                disabled: disabled.contains(name),
            })
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(v)
}

pub fn package_info(serial: &str, pkg: &str) -> Result<String, String> {
    let out = shell(serial, &format!("dumpsys package {pkg}"))?;
    let keys = [
        "versionName",
        "versionCode",
        "targetSdk",
        "minSdk",
        "firstInstallTime",
        "lastUpdateTime",
        "installerPackageName",
        "codePath",
        "dataDir",
    ];
    let mut res = String::new();
    for l in out.lines() {
        let t = l.trim();
        if keys.iter().any(|k| t.starts_with(k)) {
            res.push_str(t);
            res.push('\n');
        }
    }
    let granted: Vec<&str> = out
        .lines()
        .map(str::trim)
        .filter(|l| l.contains("granted=true") && l.contains("android.permission"))
        .collect();
    if !granted.is_empty() {
        res.push_str("\nGranted permissions:\n");
        for g in granted {
            res.push_str(g.split(':').next().unwrap_or(g));
            res.push('\n');
        }
    }
    Ok(res)
}

pub fn pull_apks(serial: &str, pkg: &str, dest: &str) -> Result<String, String> {
    let paths = shell(serial, &format!("pm path {pkg}"))?;
    let mut log = String::new();
    for p in paths.lines().filter_map(|l| l.trim().strip_prefix("package:")) {
        let file = p.rsplit('/').next().unwrap_or("base.apk");
        let local = format!("{dest}/{pkg}_{file}");
        run(Some(serial), &["pull", p, &local])?;
        log.push_str(&format!("{local}\n"));
    }
    Ok(log.trim().to_string())
}

pub fn screenshot(serial: &str, dest: &str) -> Result<String, String> {
    let out = Command::new("adb")
        .args(["-s", serial, "exec-out", "screencap", "-p"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() || out.stdout.is_empty() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    std::fs::write(dest, out.stdout).map_err(|e| e.to_string())?;
    Ok(dest.to_string())
}

fn cache_path(pkg: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".cache/rusty-adb/icons").join(format!("{pkg}.png"))
}

fn icon_score(name: &str) -> Option<i32> {
    let lower = name.to_lowercase();
    if !(lower.starts_with("res/") || lower.starts_with("r/")) {
        return None;
    }
    if !(lower.ends_with(".png") || lower.ends_with(".webp")) {
        return None;
    }
    let mut s = 0;
    if lower.contains("mipmap") {
        s += 50;
    }
    if lower.contains("launcher") {
        s += 100;
    }
    if lower.contains("round") || lower.contains("foreground") || lower.contains("background") {
        s -= 60;
    }
    if !lower.contains("mipmap") && !lower.contains("launcher") && !lower.contains("icon") {
        return None;
    }
    for (d, v) in [("xxxhdpi", 5), ("xxhdpi", 4), ("xhdpi", 3), ("hdpi", 2), ("mdpi", 1)] {
        if lower.contains(d) {
            s += v;
            break;
        }
    }
    Some(s)
}

fn best_icon_in(apk: &std::path::Path) -> Option<(i32, Vec<u8>)> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(apk).ok()?).ok()?;
    let (score, _, idx) = (0..z.len())
        .filter_map(|i| {
            let e = z.by_index(i).ok()?;
            Some((icon_score(&e.name().ok()?)?, e.size(), i))
        })
        .max_by_key(|&(s, size, _)| (s, size))?;
    let mut e = z.by_index(idx).ok()?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut e, &mut buf).ok()?;
    Some((score, buf))
}

fn read_entry(apk: &std::path::Path, name: &str) -> Option<Vec<u8>> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(apk).ok()?).ok()?;
    let mut e = z.by_name(name).ok()?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut e, &mut buf).ok()?;
    Some(buf)
}

/// Pull the app's APKs (base, then density splits) and extract its launcher icon as
/// image bytes; cached on disk. The manifest's icon is resolved first, falling back to
/// a filename heuristic. Adaptive/vector-only icons aren't supported.
pub fn fetch_icon(serial: &str, pkg: &str, _apk: &str) -> Result<Vec<u8>, String> {
    let cache = cache_path(pkg);
    if let Ok(b) = std::fs::read(&cache) {
        return Ok(b);
    }
    let paths = shell(serial, &format!("pm path {pkg}"))?;
    let mut tmps = Vec::new();
    for (n, path) in paths
        .lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .filter(|p| p.ends_with("base.apk") || p.contains("dpi"))
        .enumerate()
    {
        let tmp = std::env::temp_dir().join(format!("rusty-adb-{pkg}-{n}.apk"));
        if let Err(e) = run(Some(serial), &["pull", path, &tmp.to_string_lossy()]) {
            tmps.iter().for_each(|t| drop(std::fs::remove_file(t)));
            return Err(e);
        }
        tmps.push(tmp);
    }
    let is_raster = |n: &str| n.ends_with(".png") || n.ends_with(".webp");
    let manifest_icon = tmps.first().and_then(|base| {
        let apk = apk_info::Apk::new(base).ok()?;
        let name = apk.get_application_icon().or_else(|| apk.get_application_logo())?;
        is_raster(&name).then_some(name)
    });
    let from_manifest = manifest_icon
        .and_then(|name| tmps.iter().find_map(|t| read_entry(t, &name)));
    let found = from_manifest.or_else(|| {
        tmps.iter()
            .filter_map(|t| best_icon_in(t))
            .max_by_key(|(s, _)| *s)
            .map(|(_, b)| b)
    });
    tmps.iter().for_each(|t| drop(std::fs::remove_file(t)));
    let bytes = found.ok_or("no raster icon found")?;
    if let Some(dir) = cache.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&cache, &bytes);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn icons_live() {
        let s = super::devices().unwrap()[0].serial.clone();
        let pk = super::packages(&s).unwrap();
        let (mut ok, mut n) = (0, 0);
        for p in pk.iter().filter(|p| !p.system).take(8) {
            n += 1;
            match super::fetch_icon(&s, &p.name, &p.apk) {
                Ok(b) => { ok += 1; println!("{} ok {} bytes img={}", p.name, b.len(), image::load_from_memory(&b).is_ok()); }
                Err(e) => println!("{} ERR {e}", p.name),
            }
        }
        println!("{ok}/{n}");
    }
}

