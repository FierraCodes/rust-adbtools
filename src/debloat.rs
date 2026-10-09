//! Built-in list of commonly removed preinstalled packages.
//! Removal uses `pm uninstall --user 0`, which is reversible (Restore), but "Caution"
//! entries can break features such as calls, SMS, push notifications or updates.

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Safety {
    Safe,
    Caution,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vendor {
    Google,
    Samsung,
    Xiaomi,
    Huawei,
    Social,
    Misc,
}

impl Vendor {
    pub const ALL: [Vendor; 6] = [
        Vendor::Google,
        Vendor::Samsung,
        Vendor::Xiaomi,
        Vendor::Huawei,
        Vendor::Social,
        Vendor::Misc,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Vendor::Google => "Google",
            Vendor::Samsung => "Samsung",
            Vendor::Xiaomi => "Xiaomi",
            Vendor::Huawei => "Huawei/Honor",
            Vendor::Social => "Social & 3rd party",
            Vendor::Misc => "AOSP/misc",
        }
    }

    /// Map `ro.product.manufacturer` to the matching vendor list, if any.
    pub fn detect(manufacturer: &str) -> Option<Vendor> {
        let m = manufacturer.to_lowercase();
        if m.contains("samsung") {
            Some(Vendor::Samsung)
        } else if ["xiaomi", "redmi", "poco"].iter().any(|k| m.contains(k)) {
            Some(Vendor::Xiaomi)
        } else if ["huawei", "honor"].iter().any(|k| m.contains(k)) {
            Some(Vendor::Huawei)
        } else {
            None
        }
    }
}

pub struct Entry {
    pub pkg: &'static str,
    pub vendor: Vendor,
    pub safety: Safety,
    pub desc: &'static str,
}

const fn e(pkg: &'static str, vendor: Vendor, safety: Safety, desc: &'static str) -> Entry {
    Entry { pkg, vendor, safety, desc }
}

use Safety::{Caution as C, Safe as S};
use Vendor::{Google as G, Huawei as H, Misc as M, Samsung as Sm, Social as So, Xiaomi as X};

pub static LIST: &[Entry] = &[
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
    e("com.facebook.katana", So, S, "Facebook"),
    e("com.facebook.orca", So, S, "Messenger"),
    e("com.facebook.system", So, S, "Facebook system stub"),
    e("com.facebook.appmanager", So, S, "Facebook App Manager"),
    e("com.facebook.services", So, S, "Facebook Services"),
    e("com.instagram.android", So, S, "Instagram"),
    e("com.netflix.mediaclient", So, S, "Netflix"),
    e("com.netflix.partner.activation", So, S, "Netflix partner activation"),
    e("com.spotify.music", So, S, "Spotify"),
    e("com.amazon.mShop.android.shopping", So, S, "Amazon Shopping"),
    e("com.linkedin.android", So, S, "LinkedIn"),
    e("com.zhiliaoapp.musically", So, S, "TikTok"),
    e("com.microsoft.office.officehubrow", So, S, "Microsoft 365 (Office)"),
    e("com.microsoft.skydrive", So, S, "OneDrive"),
    e("com.microsoft.office.outlook", So, S, "Outlook"),
    e("com.microsoft.appmanager", So, S, "Link to Windows"),
    // Samsung
    e("com.samsung.android.bixby.agent", Sm, S, "Bixby"),
    e("com.samsung.android.bixby.wakeup", Sm, S, "Bixby voice wake-up"),
    e("com.samsung.android.app.spage", Sm, S, "Bixby Home / Samsung Free"),
    e("com.samsung.android.visionintelligence", Sm, S, "Bixby Vision"),
    e("com.samsung.android.game.gamehome", Sm, S, "Game Launcher"),
    e("com.samsung.android.game.gametools", Sm, S, "Game Tools"),
    e("com.samsung.android.arzone", Sm, S, "AR Zone"),
    e("com.samsung.android.ardrawing", Sm, S, "AR Doodle"),
    e("com.samsung.android.aremoji", Sm, S, "AR Emoji"),
    e("com.samsung.android.stickercenter", Sm, S, "Sticker Center"),
    e("com.samsung.android.app.tips", Sm, S, "Tips"),
    e("com.samsung.android.app.sharelive", Sm, S, "Quick Share (Share Live)"),
    e("com.samsung.android.mateagent", Sm, S, "Galaxy Friends"),
    e("com.samsung.android.kidsinstaller", Sm, S, "Samsung Kids installer"),
    e("com.samsung.android.app.watchmanagerstub", Sm, S, "Galaxy Wearable stub"),
    e("com.samsung.android.themestore", Sm, S, "Galaxy Themes"),
    e("com.sec.android.app.sbrowser", Sm, S, "Samsung Internet"),
    e("com.samsung.android.email.provider", Sm, S, "Samsung Email"),
    e("com.sec.android.easyMover", Sm, S, "Smart Switch"),
    e("com.sec.android.daemonapp", Sm, S, "Samsung Weather"),
    e("com.samsung.android.app.galaxyfinder", Sm, S, "Finder"),
    e("com.samsung.android.samsungpass", Sm, C, "Samsung Pass"),
    e("com.samsung.android.app.routines", Sm, C, "Bixby Routines / Modes"),
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
