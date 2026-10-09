//! Debloat package database.
//!
//! The primary source is the Universal Android Debloater Next Generation list
//! (https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation),
//! downloaded at runtime and cached on disk. A small built-in list is the offline fallback.

use serde::Deserialize;
use std::collections::HashMap;

pub const SOURCE_URL: &str = "https://raw.githubusercontent.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation/main/resources/assets/uad_lists.json";
const MAX_AGE_SECS: u64 = 7 * 24 * 3600;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Default)]
pub enum Removal {
    #[default]
    Recommended,
    Advanced,
    Expert,
    Unsafe,
}

impl Removal {
    pub const ALL: [Removal; 4] = [Removal::Recommended, Removal::Advanced, Removal::Expert, Removal::Unsafe];

    pub fn name(self) -> &'static str {
        match self {
            Removal::Recommended => "recommended",
            Removal::Advanced => "advanced",
            Removal::Expert => "expert",
            Removal::Unsafe => "unsafe",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default)]
pub struct Entry {
    pub list: String,
    pub description: String,
    pub dependencies: Vec<String>,
    #[serde(rename = "neededBy")]
    pub needed_by: Vec<String>,
    pub removal: Removal,
}

pub type Db = HashMap<String, Entry>;

pub fn parse(json: &str) -> Result<Db, String> {
    serde_json::from_str(json).map_err(|e| format!("bad debloat list: {e}"))
}

fn cache_file() -> std::path::PathBuf {
    crate::platform::cache_dir().join("uad_lists.json")
}

/// Cached list and whether it is old enough (or missing) to warrant a re-download.
pub fn load_cached() -> (Option<Db>, bool) {
    let path = cache_file();
    let db = std::fs::read_to_string(&path).ok().and_then(|t| parse(&t).ok());
    let stale = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_none_or(|age| age.as_secs() > MAX_AGE_SECS);
    let missing = db.is_none();
    (db, missing || stale)
}

/// Download the latest list, validate it, and cache it.
pub fn download() -> Result<Db, String> {
    let text = ureq::get(SOURCE_URL)
        .call()
        .map_err(|e| format!("download failed: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("download failed: {e}"))?;
    let db = parse(&text)?;
    let path = cache_file();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, text);
    Ok(db)
}

pub fn builtin() -> Db {
    BUILTIN
        .iter()
        .map(|&(pkg, list, removal, desc)| {
            (
                pkg.to_string(),
                Entry { list: list.into(), description: desc.into(), removal, ..Default::default() },
            )
        })
        .collect()
}

use Removal::{Advanced as C, Recommended as S};

const G: &str = "Google";
const H: &str = "Oem";
const X: &str = "Oem";
const SM: &str = "Oem";
const SO: &str = "Misc";
const M: &str = "Aosp";

const fn e(pkg: &'static str, list: &'static str, removal: Removal, desc: &'static str) -> (&'static str, &'static str, Removal, &'static str) {
    (pkg, list, removal, desc)
}

static BUILTIN: &[(&str, &str, Removal, &str)] = &[
    // Google
    e("com.google.android.apps.tachyon", G, S, "Google Meet / Duo"),
    e("com.google.android.apps.magazines", G, S, "Google News"),
    e("com.google.android.videos", G, S, "Google TV (Play Movies)"),
    e("com.google.android.music", G, S, "Play Music (discontinued)"),
    e("com.google.android.apps.podcasts", G, S, "Google Podcasts"),
    e("com.google.android.apps.youtube.music", G, S, "YouTube Music"),
    e("com.google.android.youtube", G, S, "YouTube"),
    e("com.google.android.apps.docs", G, S, "Google Drive"),
    e("com.google.android.apps.photos", G, S, "Google Photos"),
    e("com.google.android.apps.maps", G, S, "Google Maps"),
    e("com.google.android.gm", G, S, "Gmail"),
    e("com.google.android.apps.subscriptions.red", G, S, "Google One"),
    e("com.google.android.apps.wellbeing", G, S, "Digital Wellbeing"),
    e("com.google.android.apps.nbu.files", G, S, "Files by Google"),
    e("com.google.android.projection.gearhead", G, S, "Android Auto"),
    e("com.google.ar.core", G, S, "Google Play Services for AR"),
    e("com.google.android.feedback", G, S, "Market feedback agent"),
    e("com.google.android.printservice.recommendation", G, S, "Print service recommendations"),
    e("com.google.android.apps.googleassistant", G, C, "Google Assistant"),
    e("com.google.android.googlequicksearchbox", G, C, "Google app / search bar; some launchers depend on it"),
    e("com.google.android.apps.turbo", G, C, "Device Health Services (battery stats)"),
    e("com.google.android.apps.walletnfcrel", G, C, "Google Wallet / NFC payments"),
    e("com.google.android.apps.messaging", G, C, "Messages (may be your only SMS app)"),
    e("com.google.android.dialer", G, C, "Phone (may be your only dialer)"),
    e("com.google.android.partnersetup", G, C, "Partner setup"),
    // Social / third party
    e("com.facebook.katana", SO, S, "Facebook"),
    e("com.facebook.orca", SO, S, "Messenger"),
    e("com.facebook.system", SO, S, "Facebook system stub"),
    e("com.facebook.appmanager", SO, S, "Facebook App Manager"),
    e("com.facebook.services", SO, S, "Facebook Services"),
    e("com.instagram.android", SO, S, "Instagram"),
    e("com.netflix.mediaclient", SO, S, "Netflix"),
    e("com.netflix.partner.activation", SO, S, "Netflix partner activation"),
    e("com.spotify.music", SO, S, "Spotify"),
    e("com.amazon.mShop.android.shopping", SO, S, "Amazon Shopping"),
    e("com.linkedin.android", SO, S, "LinkedIn"),
    e("com.zhiliaoapp.musically", SO, S, "TikTok"),
    e("com.microsoft.office.officehubrow", SO, S, "Microsoft 365 (Office)"),
    e("com.microsoft.skydrive", SO, S, "OneDrive"),
    e("com.microsoft.office.outlook", SO, S, "Outlook"),
    e("com.microsoft.appmanager", SO, S, "Link to Windows"),
    // Samsung
    e("com.samsung.android.bixby.agent", SM, S, "Bixby"),
    e("com.samsung.android.bixby.wakeup", SM, S, "Bixby voice wake-up"),
    e("com.samsung.android.app.spage", SM, S, "Bixby Home / Samsung Free"),
    e("com.samsung.android.visionintelligence", SM, S, "Bixby Vision"),
    e("com.samsung.android.game.gamehome", SM, S, "Game Launcher"),
    e("com.samsung.android.game.gametools", SM, S, "Game Tools"),
    e("com.samsung.android.arzone", SM, S, "AR Zone"),
    e("com.samsung.android.ardrawing", SM, S, "AR Doodle"),
    e("com.samsung.android.aremoji", SM, S, "AR Emoji"),
    e("com.samsung.android.stickercenter", SM, S, "Sticker Center"),
    e("com.samsung.android.app.tips", SM, S, "Tips"),
    e("com.samsung.android.app.sharelive", SM, S, "Quick Share (Share Live)"),
    e("com.samsung.android.mateagent", SM, S, "Galaxy Friends"),
    e("com.samsung.android.kidsinstaller", SM, S, "Samsung Kids installer"),
    e("com.samsung.android.app.watchmanagerstub", SM, S, "Galaxy Wearable stub"),
    e("com.samsung.android.themestore", SM, S, "Galaxy Themes"),
    e("com.sec.android.app.sbrowser", SM, S, "Samsung Internet"),
    e("com.samsung.android.email.provider", SM, S, "Samsung Email"),
    e("com.sec.android.easyMover", SM, S, "Smart Switch"),
    e("com.sec.android.daemonapp", SM, S, "Samsung Weather"),
    e("com.samsung.android.app.galaxyfinder", SM, S, "Finder"),
    e("com.samsung.android.samsungpass", SM, C, "Samsung Pass"),
    e("com.samsung.android.app.routines", SM, C, "Bixby Routines / Modes"),
    // Xiaomi
    e("com.miui.analytics", X, S, "MIUI analytics"),
    e("com.miui.msa.global", X, S, "MIUI system ads"),
    e("com.xiaomi.mipicks", X, S, "GetApps store"),
    e("com.miui.bugreport", X, S, "Bug report"),
    e("com.miui.hybrid", X, S, "Quick apps"),
    e("com.miui.player", X, S, "Mi Music"),
    e("com.miui.videoplayer", X, S, "Mi Video"),
    e("com.xiaomi.glgm", X, S, "Games"),
    e("com.miui.yellowpage", X, S, "Yellow Pages / caller ID"),
    e("com.miui.android.fashiongallery", X, S, "Wallpaper Carousel"),
    e("com.xiaomi.midrop", X, S, "ShareMe"),
    e("com.miui.weather2", X, S, "Weather"),
    e("com.mi.globalbrowser", X, S, "Mi Browser"),
    e("com.miui.mishare.connectivity", X, S, "Mi Share"),
    e("com.miui.cloudservice", X, C, "Mi Cloud"),
    e("com.miui.cloudbackup", X, C, "Mi Cloud backup"),
    e("com.miui.daemon", X, C, "MIUI daemon"),
    e("com.xiaomi.discover", X, C, "Xiaomi app discovery / updates"),
    // Huawei / Honor
    e("com.huawei.appmarket", H, S, "AppGallery"),
    e("com.huawei.music", H, S, "Huawei Music"),
    e("com.huawei.himovie.overseas", H, S, "Huawei Video"),
    e("com.huawei.browser", H, S, "Huawei Browser"),
    e("com.huawei.gamebox", H, S, "Huawei Games"),
    e("com.huawei.hwvplayer.youku", H, S, "Youku video"),
    e("com.huawei.android.tips", H, S, "Tips"),
    e("com.huawei.phoneservice", H, S, "HiCare"),
    e("com.huawei.android.thememanager", H, S, "Themes"),
    e("com.huawei.android.totemweather", H, S, "Weather"),
    e("com.huawei.parentcontrol", H, S, "Parental controls"),
    e("com.huawei.vassistant", H, S, "Huawei Assistant (Celia)"),
    e("com.huawei.hitouch", H, S, "HiTouch"),
    e("com.huawei.scanner", H, S, "Scanner / HiVision"),
    e("com.huawei.health", H, S, "Huawei Health"),
    e("com.huawei.hwid", H, C, "Huawei ID / HMS Core account"),
    e("com.huawei.hiai", H, C, "HiAI engine (camera/AI features)"),
    e("com.huawei.android.hsf", H, C, "Huawei Services Framework"),
    e("com.huawei.trustspace", H, C, "Phone Manager / security"),
    // AOSP / misc
    e("com.android.egg", M, S, "Android easter egg"),
    e("com.android.bips", M, S, "Default print service"),
    e("com.android.printspooler", M, S, "Print spooler"),
    e("com.android.bookmarkprovider", M, S, "Bookmark provider"),
    e("com.android.providers.partnerbookmarks", M, S, "Partner bookmarks"),
    e("com.android.dreams.basic", M, S, "Basic screensaver"),
    e("com.android.dreams.phototable", M, S, "Photo screensaver"),
    e("com.android.traceur", M, S, "System tracing"),
    e("com.android.htmlviewer", M, S, "HTML viewer"),
    e("com.android.hotwordenrollment.okgoogle", M, S, "\"OK Google\" enrollment"),
    e("com.android.chrome", M, S, "Chrome (only if you have another browser)"),
    e("com.android.stk", M, C, "SIM toolkit (some carriers need it)"),
    e("com.android.cellbroadcastreceiver", M, C, "Emergency / cell broadcast alerts"),
];

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn live_download_matches_device() {
        let db = super::download().unwrap();
        let s = crate::adb::devices().unwrap()[0].serial.clone();
        let pk = crate::adb::packages(&s).unwrap();
        let hits: Vec<_> = pk.iter().filter_map(|p| db.get(&p.name).map(|e| (&p.name, e.removal))).collect();
        println!("db {} entries, device {} pkgs, {} known", db.len(), pk.len(), hits.len());
        for r in super::Removal::ALL {
            println!("{r:?}: {}", hits.iter().filter(|h| h.1 == r).count());
        }
    }
}
