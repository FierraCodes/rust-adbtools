mod adb;
mod debloat;

use adb::{Device, Package};
use debloat::Removal;
use eframe::egui::{self, Color32, RichText};
use std::collections::{BTreeSet, HashMap};
use std::process::{Child, Command};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

enum Msg {
    Log(String),
    Devices(Vec<Device>),
    Packages(Vec<Package>),
    Info(String),
    Shell(String),
    Busy(bool),
    Icon(String, Option<egui::ColorImage>),
    Manufacturer(String),
    Db(Result<debloat::Db, String>),
}

enum IconState {
    Pending,
    Ready(egui::TextureHandle),
    Missing,
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Apps,
    Mirror,
    Debloat,
    Tools,
    Shell,
}

#[derive(PartialEq, Clone, Copy)]
enum Filter {
    All,
    User,
    System,
    Disabled,
    Removed,
}

#[derive(Clone, Copy, PartialEq)]
enum Bulk {
    Uninstall,
    Disable,
    Enable,
    Restore,
    Clear,
    Stop,
    Pull,
}

struct Scrcpy {
    max_size: String,
    bitrate: String,
    max_fps: String,
    record: String,
    title: String,
    no_audio: bool,
    no_control: bool,
    screen_off: bool,
    stay_awake: bool,
    on_top: bool,
    fullscreen: bool,
    show_touches: bool,
    child: Option<Child>,
}

struct App {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    tab: Tab,
    devices: Vec<Device>,
    serial: Option<String>,
    packages: Vec<Package>,
    search: String,
    filter: Filter,
    selected: BTreeSet<String>,
    focus: Option<String>,
    info: String,
    keep_data: bool,
    confirm: Option<Bulk>,
    log: Vec<String>,
    busy: bool,
    connect_addr: String,
    pair_addr: String,
    pair_code: String,
    apk_path: String,
    out_dir: String,
    shell_cmd: String,
    shell_out: String,
    scrcpy: Scrcpy,
    db: debloat::Db,
    db_status: String,
    deb_list: Option<String>,
    deb_levels: [bool; 4],
    deb_search: String,
    deb_disable: bool,
    deb_sel: BTreeSet<String>,
    deb_confirm: bool,
    manufacturer: String,
    need_refresh: bool,
    icons: HashMap<String, IconState>,
    icon_tx: Sender<(String, String, String)>,
    ctx: egui::Context,
    logo: egui::TextureHandle,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        ctx.set_visuals(egui::Visuals::dark());
        let (tx, rx) = channel();
        let (icon_tx, icon_rx) = channel::<(String, String, String)>();
        {
            let tx = tx.clone();
            let ctx = ctx.clone();
            thread::spawn(move || {
                while let Ok((serial, pkg, apk)) = icon_rx.recv() {
                    let img = adb::fetch_icon(&serial, &pkg, &apk)
                        .ok()
                        .and_then(|b| image::load_from_memory(&b).ok())
                        .map(|i| {
                            let i = i.thumbnail(96, 96).to_rgba8();
                            let size = [i.width() as usize, i.height() as usize];
                            egui::ColorImage::from_rgba_unmultiplied(size, i.as_raw())
                        });
                    let _ = tx.send(Msg::Icon(pkg, img));
                    ctx.request_repaint();
                }
            });
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let mut app = App {
            tx,
            rx,
            tab: Tab::Apps,
            devices: vec![],
            serial: None,
            packages: vec![],
            search: String::new(),
            filter: Filter::User,
            selected: BTreeSet::new(),
            focus: None,
            info: String::new(),
            keep_data: false,
            confirm: None,
            log: vec![],
            busy: false,
            connect_addr: String::new(),
            pair_addr: String::new(),
            pair_code: String::new(),
            apk_path: String::new(),
            out_dir: home,
            shell_cmd: String::new(),
            shell_out: String::new(),
            db: debloat::builtin(),
            db_status: "built-in list (offline)".into(),
            deb_list: None,
            deb_levels: [true, true, false, false],
            deb_search: String::new(),
            deb_disable: false,
            deb_sel: BTreeSet::new(),
            deb_confirm: false,
            manufacturer: String::new(),
            need_refresh: false,
            icons: HashMap::new(),
            icon_tx,
            ctx: ctx.clone(),
            logo: {
                let i = app_icon();
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [i.width as usize, i.height as usize],
                    &i.rgba,
                );
                ctx.load_texture("logo", img, egui::TextureOptions::LINEAR)
            },
            scrcpy: Scrcpy {
                max_size: String::new(),
                bitrate: String::new(),
                max_fps: String::new(),
                record: String::new(),
                title: String::new(),
                no_audio: false,
                no_control: false,
                screen_off: false,
                stay_awake: true,
                on_top: false,
                fullscreen: false,
                show_touches: false,
                child: None,
            },
        };
        app.refresh_devices();
        let (cached, stale) = debloat::load_cached();
        if let Some(db) = cached {
            app.db_status = format!("UAD list (cached), {} entries", db.len());
            app.db = db;
        }
        if stale {
            app.update_db();
        }
        app
    }

    fn update_db(&mut self) {
        self.db_status = "downloading list…".into();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(Msg::Db(debloat::download()));
            ctx.request_repaint();
        });
    }

    /// Run `f` on a worker thread; it can send messages back through `tx`.
    fn spawn<F: FnOnce(Sender<Msg>) + Send + 'static>(&mut self, ctx: &egui::Context, f: F) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(Msg::Busy(true));
            f(tx.clone());
            let _ = tx.send(Msg::Busy(false));
            ctx.request_repaint();
        });
    }

    fn refresh_devices(&mut self) {
        let tx = self.tx.clone();
        thread::spawn(move || match adb::devices() {
            Ok(d) => {
                let _ = tx.send(Msg::Devices(d));
            }
            Err(e) => {
                let _ = tx.send(Msg::Log(format!("devices: {e}")));
            }
        });
    }

    fn refresh_packages(&mut self, ctx: &egui::Context) {
        let Some(s) = self.serial.clone() else { return };
        self.spawn(ctx, move |tx| {
            if let Ok(m) = adb::shell(&s, "getprop ro.product.manufacturer") {
                let _ = tx.send(Msg::Manufacturer(m));
            }
            match adb::packages(&s) {
                Ok(p) => {
                    let _ = tx.send(Msg::Packages(p));
                }
                Err(e) => {
                    let _ = tx.send(Msg::Log(format!("packages: {e}")));
                }
            }
        });
    }

    /// Draw a package's icon, queueing a background fetch the first time it's seen.
    fn icon(&mut self, ui: &mut egui::Ui, p: &Package, size: f32) {
        if !self.icons.contains_key(&p.name) {
            if let Some(s) = &self.serial {
                let _ = self.icon_tx.send((s.clone(), p.name.clone(), p.apk.clone()));
            }
            self.icons.insert(p.name.clone(), IconState::Pending);
        }
        let sz = egui::vec2(size, size);
        match self.icons.get(&p.name) {
            Some(IconState::Ready(t)) => {
                ui.add(egui::Image::new((t.id(), sz)));
            }
            Some(IconState::Pending) => {
                ui.add_sized(sz, egui::Spinner::new().size(size * 0.6));
            }
            _ => {
                ui.add_sized(sz, egui::Label::new("📦"));
            }
        }
    }

    fn poll(&mut self) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Log(l) => self.log.push(l),
                Msg::Devices(d) => {
                    let keep = self
                        .serial
                        .as_ref()
                        .is_some_and(|s| d.iter().any(|x| &x.serial == s));
                    if !keep {
                        self.serial = d
                            .iter()
                            .find(|x| x.state == "device")
                            .map(|x| x.serial.clone());
                        self.packages.clear();
                        self.selected.clear();
                        self.icons.clear();
                        self.need_refresh = self.serial.is_some();
                    }
                    self.devices = d;
                }
                Msg::Packages(p) => {
                    self.selected.retain(|s| p.iter().any(|x| &x.name == s));
                    self.packages = p;
                }
                Msg::Info(i) => self.info = i,
                Msg::Shell(o) => self.shell_out = o,
                Msg::Busy(b) => self.busy = b,
                Msg::Manufacturer(m) => {
                    self.manufacturer = m;
                }
                Msg::Db(r) => match r {
                    Ok(db) => {
                        self.db_status = format!("UAD list, {} entries", db.len());
                        self.db = db;
                    }
                    Err(e) => self.log.push(format!("debloat list: {e}")),
                },
                Msg::Icon(pkg, img) => {
                    let st = match img {
                        Some(i) => IconState::Ready(self.ctx.load_texture(&pkg, i, Default::default())),
                        None => IconState::Missing,
                    };
                    self.icons.insert(pkg, st);
                }
            }
        }
        if let Some(c) = &mut self.scrcpy.child {
            if let Ok(Some(st)) = c.try_wait() {
                self.log.push(format!("scrcpy exited ({st})"));
                self.scrcpy.child = None;
            }
        }
    }

    fn run_bulk(&mut self, ctx: &egui::Context, op: Bulk, pkgs: Vec<String>) {
        let Some(s) = self.serial.clone() else { return };
        let keep = self.keep_data;
        let out = self.out_dir.clone();
        self.spawn(ctx, move |tx| {
            for p in &pkgs {
                let r = match op {
                    Bulk::Uninstall => adb::shell(
                        &s,
                        &format!("pm uninstall --user 0 {} {p}", if keep { "-k" } else { "" }),
                    ),
                    Bulk::Disable => adb::shell(&s, &format!("pm disable-user --user 0 {p}")),
                    Bulk::Enable => adb::shell(&s, &format!("pm enable {p}")),
                    Bulk::Restore => adb::shell(&s, &format!("cmd package install-existing {p}")),
                    Bulk::Clear => adb::shell(&s, &format!("pm clear {p}")),
                    Bulk::Stop => adb::shell(&s, &format!("am force-stop {p}")),
                    Bulk::Pull => adb::pull_apks(&s, p, &out),
                };
                if r.is_ok() {
                    let action = match op {
                        Bulk::Uninstall => Some("uninstall"),
                        Bulk::Disable => Some("disable"),
                        Bulk::Enable => Some("enable"),
                        Bulk::Restore => Some("restore"),
                        _ => None,
                    };
                    if let Some(a) = action {
                        adb::log_action(&s, p, a);
                    }
                }
                let _ = tx.send(Msg::Log(match r {
                    Ok(o) => format!("{p}: {o}"),
                    Err(e) => format!("{p}: ERROR {e}"),
                }));
            }
            if let Ok(pk) = adb::packages(&s) {
                let _ = tx.send(Msg::Packages(pk));
            }
        });
    }

    fn launch_scrcpy(&mut self) {
        let Some(s) = &self.serial else { return };
        let c = &self.scrcpy;
        let mut cmd = Command::new("scrcpy");
        cmd.args(["-s", s]);
        for (flag, val) in [
            ("--max-size", &c.max_size),
            ("--video-bit-rate", &c.bitrate),
            ("--max-fps", &c.max_fps),
            ("--record", &c.record),
            ("--window-title", &c.title),
        ] {
            if !val.trim().is_empty() {
                cmd.arg(flag).arg(val.trim());
            }
        }
        for (on, flag) in [
            (c.no_audio, "--no-audio"),
            (c.no_control, "--no-control"),
            (c.screen_off, "--turn-screen-off"),
            (c.stay_awake, "--stay-awake"),
            (c.on_top, "--always-on-top"),
            (c.fullscreen, "--fullscreen"),
            (c.show_touches, "--show-touches"),
        ] {
            if on {
                cmd.arg(flag);
            }
        }
        match cmd.spawn() {
            Ok(ch) => {
                self.log.push("scrcpy started".into());
                self.scrcpy.child = Some(ch);
            }
            Err(e) => self.log.push(format!("scrcpy: {e}")),
        }
    }

    fn adb_task(&mut self, ctx: &egui::Context, label: &str, args: Vec<String>, serial: bool) {
        let s = if serial { self.serial.clone() } else { None };
        if serial && s.is_none() {
            return;
        }
        let label = label.to_string();
        self.spawn(ctx, move |tx| {
            let a: Vec<&str> = args.iter().map(String::as_str).collect();
            let _ = tx.send(Msg::Log(match adb::run(s.as_deref(), &a) {
                Ok(o) => format!("{label}: {o}"),
                Err(e) => format!("{label}: ERROR {e}"),
            }));
        });
    }

    fn apps_tab(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.text_edit_singleline(&mut self.search);
            for (f, n) in [
                (Filter::User, "User"),
                (Filter::System, "System"),
                (Filter::Disabled, "Disabled"),
                (Filter::Removed, "Removed"),
                (Filter::All, "All"),
            ] {
                ui.selectable_value(&mut self.filter, f, n);
            }
            if ui.button("⟳ Refresh").clicked() {
                self.refresh_packages(&ctx);
            }
        });
        let q = self.search.to_lowercase();
        let visible: Vec<Package> = self
            .packages
            .iter()
            .filter(|p| match self.filter {
                Filter::All => true,
                Filter::User => !p.system,
                Filter::System => p.system,
                Filter::Disabled => p.disabled,
                Filter::Removed => p.removed,
            })
            .filter(|p| q.is_empty() || p.name.to_lowercase().contains(&q))
            .cloned()
            .collect();

        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{} shown, {} selected", visible.len(), self.selected.len()));
            if ui.small_button("Select shown").clicked() {
                self.selected.extend(visible.iter().map(|p| p.name.clone()));
            }
            if ui.small_button("Clear selection").clicked() {
                self.selected.clear();
            }
        });
        ui.horizontal_wrapped(|ui| {
            let has = !self.selected.is_empty() && self.serial.is_some();
            ui.add_enabled_ui(has, |ui| {
                for (op, name) in [
                    (Bulk::Uninstall, "🗑 Uninstall"),
                    (Bulk::Disable, "Disable"),
                    (Bulk::Enable, "Enable"),
                    (Bulk::Restore, "Restore"),
                    (Bulk::Clear, "Clear data"),
                    (Bulk::Stop, "Force stop"),
                    (Bulk::Pull, "Pull APK"),
                ] {
                    if ui.button(name).clicked() {
                        if matches!(op, Bulk::Uninstall | Bulk::Clear) {
                            self.confirm = Some(op);
                        } else {
                            { let p = self.selected.iter().cloned().collect(); self.run_bulk(&ctx, op, p); }
                        }
                    }
                }
            });
            ui.checkbox(&mut self.keep_data, "keep data on uninstall");
        });
        ui.separator();

        let row_h = 22.0 + ui.spacing().item_spacing.y;
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(
            ui,
            row_h,
            visible.len(),
            |ui, range| {
                for p in &visible[range] {
                    ui.horizontal(|ui| {
                        let mut sel = self.selected.contains(&p.name);
                        if ui.checkbox(&mut sel, "").changed() {
                            if sel {
                                self.selected.insert(p.name.clone());
                            } else {
                                self.selected.remove(&p.name);
                            }
                        }
                        self.icon(ui, p, 22.0);
                        let mut t = RichText::new(&p.name);
                        if p.removed {
                            t = t.color(Color32::from_rgb(200, 90, 90)).strikethrough();
                        } else if p.disabled {
                            t = t.color(Color32::GRAY).strikethrough();
                        } else if p.system {
                            t = t.color(Color32::from_rgb(230, 190, 110));
                        }
                        let focused = self.focus.as_deref() == Some(&p.name);
                        if ui.selectable_label(focused, t).clicked() {
                            self.focus = Some(p.name.clone());
                            self.info = "loading…".into();
                            if let Some(s) = self.serial.clone() {
                                let n = p.name.clone();
                                self.spawn(&ctx, move |tx| {
                                    let _ = tx.send(Msg::Info(
                                        adb::package_info(&s, &n).unwrap_or_else(|e| e),
                                    ));
                                });
                            }
                        }
                    });
                }
            },
        );
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let Some(pkg) = self.focus.clone() else {
            ui.label("Click a package for details.");
            return;
        };
        if let Some(p) = self.packages.iter().find(|p| p.name == pkg).cloned() {
            self.icon(ui, &p, 64.0);
            ui.heading(&pkg);
            ui.label(&p.apk);
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("▶ Launch").clicked() {
                self.adb_task(
                    &ctx,
                    "launch",
                    ["shell", "monkey", "-p", &pkg, "-c", "android.intent.category.LAUNCHER", "1"]
                        .map(String::from)
                        .to_vec(),
                    true,
                );
            }
            if ui.button("⚙ App settings").clicked() {
                self.adb_task(
                    &ctx,
                    "settings",
                    [
                        "shell", "am", "start", "-a",
                        "android.settings.APPLICATION_DETAILS_SETTINGS", "-d",
                        &format!("package:{pkg}"),
                    ]
                    .map(String::from)
                    .to_vec(),
                    true,
                );
            }
            if ui.button("📋 Copy name").clicked() {
                ui.ctx().copy_text(pkg.clone());
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().id_salt("info").show(ui, |ui| {
            ui.monospace(&self.info);
        });
    }

    fn debloat_tab(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.packages.is_empty() {
            ui.label("No package list yet. Select a device and press Refresh on the Apps tab.");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "Device: {} · {}",
                if self.manufacturer.is_empty() { "?" } else { &self.manufacturer },
                self.db_status
            ));
            if ui.small_button("⟳ Update list").clicked() {
                self.update_db();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("🔍");
            ui.add(egui::TextEdit::singleline(&mut self.deb_search).desired_width(160.0));
            egui::ComboBox::from_id_salt("dlist")
                .selected_text(self.deb_list.clone().unwrap_or_else(|| "All lists".into()))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.deb_list, None, "All lists");
                    for l in ["Oem", "Google", "Aosp", "Carrier", "Misc"] {
                        ui.selectable_value(&mut self.deb_list, Some(l.to_string()), l);
                    }
                });
            for (i, r) in Removal::ALL.iter().enumerate() {
                ui.checkbox(&mut self.deb_levels[i], RichText::new(r.name()).color(removal_color(*r)));
            }
            ui.separator();
            ui.radio_value(&mut self.deb_disable, false, "Uninstall (user 0)");
            ui.radio_value(&mut self.deb_disable, true, "Disable");
        });

        // Known packages present on this device and not already removed.
        let q = self.deb_search.to_lowercase();
        let mut rows: Vec<(String, debloat::Entry, Package)> = self
            .packages
            .iter()
            .filter_map(|p| {
                let e = self.db.get(&p.name)?;
                let active = if self.deb_disable { !p.disabled && !p.removed } else { !p.removed };
                let list_ok = self.deb_list.as_ref().is_none_or(|l| *l == e.list);
                let level_ok = self.deb_levels[e.removal as usize];
                let search_ok = q.is_empty()
                    || p.name.to_lowercase().contains(&q)
                    || e.description.to_lowercase().contains(&q);
                (active && list_ok && level_ok && search_ok).then(|| (p.name.clone(), e.clone(), p.clone()))
            })
            .collect();
        rows.sort_by(|a, b| a.1.removal.cmp(&b.1.removal).then(a.0.cmp(&b.0)));
        let visible: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
        self.deb_sel.retain(|s| visible.contains(&s.as_str()));

        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{} matches, {} selected", rows.len(), self.deb_sel.len()));
            if ui.small_button("Select all recommended").clicked() {
                self.deb_sel.extend(
                    rows.iter().filter(|r| r.1.removal == Removal::Recommended).map(|r| r.0.clone()),
                );
            }
            if ui.small_button("Clear").clicked() {
                self.deb_sel.clear();
            }
            if ui.small_button("Export selection…").clicked() {
                if let Some(f) = rfd::FileDialog::new().set_file_name("debloat.txt").save_file() {
                    let txt: Vec<&str> = self.deb_sel.iter().map(String::as_str).collect();
                    self.log.push(match std::fs::write(&f, txt.join("\n")) {
                        Ok(_) => format!("exported {} packages to {}", txt.len(), f.display()),
                        Err(e) => format!("export: {e}"),
                    });
                }
            }
            if ui.small_button("Import list…").clicked() {
                if let Some(f) = rfd::FileDialog::new().pick_file() {
                    match std::fs::read_to_string(&f) {
                        Ok(t) => {
                            let n0 = self.deb_sel.len();
                            self.deb_sel.extend(
                                t.lines().map(str::trim).filter(|l| visible.contains(l)).map(String::from),
                            );
                            self.log.push(format!("imported {} matching packages", self.deb_sel.len() - n0));
                        }
                        Err(e) => self.log.push(format!("import: {e}")),
                    }
                }
            }
            let label = if self.deb_disable { "Disable selected" } else { "Uninstall selected" };
            if ui
                .add_enabled(!self.deb_sel.is_empty(), egui::Button::new(RichText::new(label).strong()))
                .clicked()
            {
                self.deb_confirm = true;
            }
        });
        ui.label(
            RichText::new("Removal is per-user and reversible: Apps → Removed → Restore. Actions are logged to ~/.config/rusty-adb/actions.log")
                .small()
                .weak(),
        );
        ui.separator();

        let row_h = 22.0 + ui.spacing().item_spacing.y;
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_h, rows.len(), |ui, range| {
            for (name, e, p) in &rows[range] {
                ui.horizontal(|ui| {
                    let mut sel = self.deb_sel.contains(name);
                    if ui.checkbox(&mut sel, "").changed() {
                        if sel {
                            self.deb_sel.insert(name.clone());
                        } else {
                            self.deb_sel.remove(name);
                        }
                    }
                    self.icon(ui, p, 22.0);
                    ui.label(RichText::new(e.removal.name()).color(removal_color(e.removal)).small());
                    let tip = format!(
                        "{}\n\n[{}]{}{}",
                        e.description,
                        e.list,
                        if e.dependencies.is_empty() { String::new() } else { format!("\nDepends on: {}", e.dependencies.join(", ")) },
                        if e.needed_by.is_empty() { String::new() } else { format!("\nNeeded by: {}", e.needed_by.join(", ")) },
                    );
                    ui.label(name).on_hover_text(tip);
                    let first = e.description.lines().next().unwrap_or("");
                    ui.label(RichText::new(first).weak().small());
                });
            }
        });

        if self.deb_confirm {
            let n = self.deb_sel.len();
            let count = |r: Removal| {
                self.deb_sel.iter().filter(|s| self.db.get(*s).is_some_and(|e| e.removal == r)).count()
            };
            let (expert, unsafe_) = (count(Removal::Expert), count(Removal::Unsafe));
            egui::Window::new("Confirm debloat")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(format!(
                        "{} {n} package(s)?",
                        if self.deb_disable { "Disable" } else { "Uninstall for user 0:" }
                    ));
                    if expert > 0 {
                        ui.colored_label(removal_color(Removal::Expert), format!("{expert} 'expert' package(s) can break features."));
                    }
                    if unsafe_ > 0 {
                        ui.colored_label(removal_color(Removal::Unsafe), format!("{unsafe_} 'unsafe' package(s) can cause a bootloop!"));
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            self.deb_confirm = false;
                            let op = if self.deb_disable { Bulk::Disable } else { Bulk::Uninstall };
                            let sel: Vec<String> = self.deb_sel.iter().cloned().collect();
                            self.deb_sel.clear();
                            self.run_bulk(&ctx, op, sel);
                        }
                        if ui.button("Cancel").clicked() {
                            self.deb_confirm = false;
                        }
                    });
                });
        }
    }

    fn mirror_tab(&mut self, ui: &mut egui::Ui) {
        let c = &mut self.scrcpy;
        egui::Grid::new("scrcpy").num_columns(2).show(ui, |ui| {
            for (label, val, hint) in [
                ("Max size (px)", &mut c.max_size, "e.g. 1024"),
                ("Video bitrate", &mut c.bitrate, "e.g. 8M"),
                ("Max FPS", &mut c.max_fps, "e.g. 60"),
                ("Record to file", &mut c.record, "e.g. out.mp4"),
                ("Window title", &mut c.title, ""),
            ] {
                ui.label(label);
                ui.add(egui::TextEdit::singleline(val).hint_text(hint));
                ui.end_row();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut c.no_audio, "No audio");
            ui.checkbox(&mut c.no_control, "View only");
            ui.checkbox(&mut c.screen_off, "Turn screen off");
            ui.checkbox(&mut c.stay_awake, "Stay awake");
            ui.checkbox(&mut c.on_top, "Always on top");
            ui.checkbox(&mut c.fullscreen, "Fullscreen");
            ui.checkbox(&mut c.show_touches, "Show touches");
        });
        ui.add_space(8.0);
        let running = c.child.is_some();
        ui.horizontal(|ui| {
            if ui.add_enabled(!running && self.serial.is_some(), egui::Button::new("▶ Start scrcpy")).clicked() {
                self.launch_scrcpy();
            }
            if ui.add_enabled(running, egui::Button::new("■ Stop")).clicked() {
                if let Some(ch) = &mut self.scrcpy.child {
                    let _ = ch.kill();
                }
            }
        });
    }

    fn tools_tab(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.group(|ui| {
            ui.strong("Wireless");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.connect_addr).hint_text("ip:port"));
                if ui.button("Connect").clicked() {
                    let a = self.connect_addr.clone();
                    self.adb_task(&ctx, "connect", vec!["connect".into(), a], false);
                    self.refresh_devices();
                }
                if ui.button("Disconnect all").clicked() {
                    self.adb_task(&ctx, "disconnect", vec!["disconnect".into()], false);
                    self.refresh_devices();
                }
                if ui.button("Enable TCP/IP (5555)").clicked() {
                    self.adb_task(&ctx, "tcpip", vec!["tcpip".into(), "5555".into()], true);
                }
            });
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.pair_addr).hint_text("pair ip:port"));
                ui.add(egui::TextEdit::singleline(&mut self.pair_code).hint_text("pairing code").desired_width(100.0));
                if ui.button("Pair").clicked() {
                    let (a, c) = (self.pair_addr.clone(), self.pair_code.clone());
                    self.adb_task(&ctx, "pair", vec!["pair".into(), a, c], false);
                }
            });
        });
        ui.group(|ui| {
            ui.strong("Install APK(s)");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.apk_path).hint_text("path to .apk"));
                if ui.button("Browse…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().add_filter("APK", &["apk"]).pick_file() {
                        self.apk_path = p.display().to_string();
                    }
                }
                if ui.add_enabled(!self.apk_path.is_empty(), egui::Button::new("Install")).clicked() {
                    let a = self.apk_path.clone();
                    self.adb_task(&ctx, "install", ["install", "-r", "-d"].map(String::from).into_iter().chain([a]).collect(), true);
                }
            });
        });
        ui.group(|ui| {
            ui.strong("Output folder (APK pulls, screenshots)");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.out_dir);
                if ui.button("Browse…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        self.out_dir = p.display().to_string();
                    }
                }
            });
            if ui.button("📷 Screenshot").clicked() {
                if let Some(s) = self.serial.clone() {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let dest = format!("{}/screenshot_{ts}.png", self.out_dir);
                    self.spawn(&ctx, move |tx| {
                        let _ = tx.send(Msg::Log(match adb::screenshot(&s, &dest) {
                            Ok(p) => format!("saved {p}"),
                            Err(e) => format!("screenshot: ERROR {e}"),
                        }));
                    });
                }
            }
        });
        ui.group(|ui| {
            ui.strong("Power");
            ui.horizontal(|ui| {
                for (n, a) in [
                    ("Reboot", vec!["reboot"]),
                    ("Recovery", vec!["reboot", "recovery"]),
                    ("Bootloader", vec!["reboot", "bootloader"]),
                    ("Sideload", vec!["reboot", "sideload"]),
                ] {
                    if ui.button(n).clicked() {
                        self.adb_task(&ctx, n, a.into_iter().map(String::from).collect(), true);
                    }
                }
            });
        });
    }

    fn shell_tab(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let mut run = false;
        ui.horizontal(|ui| {
            ui.label("adb shell");
            let r = ui.add(egui::TextEdit::singleline(&mut self.shell_cmd).desired_width(f32::INFINITY));
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                run = true;
                r.request_focus();
            }
        });
        if run && !self.shell_cmd.trim().is_empty() {
            if let Some(s) = self.serial.clone() {
                let c = self.shell_cmd.clone();
                self.spawn(&ctx, move |tx| {
                    let _ = tx.send(Msg::Shell(adb::shell(&s, &c).unwrap_or_else(|e| e)));
                });
            }
        }
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.monospace(&self.shell_out);
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll();
        if self.need_refresh {
            self.need_refresh = false;
            self.refresh_packages(&ctx);
        }
        if self.scrcpy.child.is_some() || self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }

        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Image::new((self.logo.id(), egui::vec2(24.0, 24.0))));
                ui.heading("Rusty ADB");
                ui.separator();
                let cur = self
                    .devices
                    .iter()
                    .find(|d| Some(&d.serial) == self.serial.as_ref())
                    .map(Device::label)
                    .unwrap_or_else(|| "no device".into());
                let mut changed = false;
                egui::ComboBox::from_id_salt("dev").selected_text(cur).width(300.0).show_ui(ui, |ui| {
                    for d in &self.devices {
                        if ui
                            .selectable_value(&mut self.serial, Some(d.serial.clone()), d.label())
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
                if ui.button("⟳").on_hover_text("Refresh devices").clicked() {
                    self.refresh_devices();
                }
                if changed {
                    self.packages.clear();
                    self.selected.clear();
                    self.focus = None;
                    self.refresh_packages(&ctx);
                }
                if self.busy {
                    ui.spinner();
                }
            });
            ui.horizontal(|ui| {
                for (t, n) in [
                    (Tab::Apps, "📦 Apps"),
                    (Tab::Mirror, "🖥 Mirror"),
                    (Tab::Debloat, "🗑 Debloat"),
                    (Tab::Tools, "🔧 Tools"),
                    (Tab::Shell, "⌨ Shell"),
                ] {
                    ui.selectable_value(&mut self.tab, t, n);
                }
            });
        });

        egui::Panel::bottom("log").resizable(true).default_size(110.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Log");
                if ui.small_button("clear").clicked() {
                    self.log.clear();
                }
            });
            egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink(false).show(ui, |ui| {
                for l in &self.log {
                    ui.monospace(l);
                }
            });
        });

        if self.tab == Tab::Apps {
            egui::Panel::right("side").default_size(320.0).show(ui, |ui| self.side_panel(ui));
        }

        egui::CentralPanel::default().show(ui, |ui| match self.tab {
            Tab::Apps => self.apps_tab(ui),
            Tab::Mirror => self.mirror_tab(ui),
            Tab::Debloat => self.debloat_tab(ui),
            Tab::Tools => self.tools_tab(ui),
            Tab::Shell => self.shell_tab(ui),
        });

        if let Some(op) = self.confirm {
            let n = self.selected.len();
            egui::Window::new("Confirm")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(match op {
                        Bulk::Uninstall => format!("Uninstall {n} package(s) for user 0?"),
                        _ => format!("Clear data of {n} package(s)? This cannot be undone."),
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            self.confirm = None;
                            { let p = self.selected.iter().cloned().collect(); self.run_bulk(&ctx, op, p); }
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm = None;
                        }
                    });
                });
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(c) = &mut self.scrcpy.child {
            let _ = c.kill();
        }
    }
}

fn removal_color(r: Removal) -> Color32 {
    match r {
        Removal::Recommended => Color32::from_rgb(110, 200, 120),
        Removal::Advanced => Color32::from_rgb(235, 190, 70),
        Removal::Expert => Color32::from_rgb(235, 130, 60),
        Removal::Unsafe => Color32::from_rgb(230, 80, 80),
    }
}

fn app_icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("embedded icon is valid")
        .to_rgba8();
    egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() }
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_app_id("rusty-adb")
            .with_icon(app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Rusty ADB",
        opts,
        Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx)))),
    )
}
