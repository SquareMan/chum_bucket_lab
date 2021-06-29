use druid::{
    widget::{Button, Checkbox, Flex, Label, LineBreaking, List, ListIter, Scroll},
    AppDelegate, DelegateCtx, EventCtx, Handled,
};
use druid::{
    Color, Command, Env, FileDialogOptions, FileSpec, Lens, LocalizedString, Target, Widget,
    WidgetExt,
};

use crate::data::{self, PATH_ROM};
use crate::data::{AppData, Mod, Patch, Rom};

const PANEL_SPACING: f64 = 10.0;
const LABEL_SPACING: f64 = 5.0;
const BG_COLOR: Color = Color::grey8(0x80);

impl ListIter<(AppData, Mod, bool)> for AppData {
    fn for_each(&self, mut cb: impl FnMut(&(AppData, Mod, bool), usize)) {
        for (i, item) in self.enabled_mods.iter().enumerate() {
            cb(&(self.clone(), self.modlist[i].clone(), item.to_owned()), i)
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut (AppData, Mod, bool), usize)) {
        let self_clone = self.clone();

        for (i, item) in self.enabled_mods.iter_mut().enumerate() {
            let mut data = (self_clone.clone(), self.modlist[i].clone(), *item);
            cb(&mut data, i);

            // Update this mod's enabled status
            *item = data.2;

            // Update selected mod
            // We can't just blindly assign to self.selected_mod because subsequent iterations will
            // begin with the previous value and then overwrite the new value
            if self_clone.selected_mod != data.0.selected_mod {
                self.selected_mod = data.0.selected_mod;
            }
        }
    }

    fn data_len(&self) -> usize {
        self.enabled_mods.data_len()
    }
}

struct ModLens;

impl Lens<(AppData, Mod, bool), bool> for ModLens {
    fn with<R, F: FnOnce(&bool) -> R>(&self, data: &(AppData, Mod, bool), f: F) -> R {
        f(&data.2)
    }

    fn with_mut<R, F: FnOnce(&mut bool) -> R>(&self, data: &mut (AppData, Mod, bool), f: F) -> R {
        f(&mut data.2)
    }
}

pub fn ui_builder() -> impl Widget<AppData> {
    // build mod panel
    let modlist_panel = Scroll::new(List::new(|| {
        Flex::row()
            .with_child(Checkbox::new("").lens(ModLens))
            .with_child(
                Label::new(|(_, m, _): &(AppData, Mod, bool), _: &Env| m.name.to_string())
                    .on_click(|_, (a, m, _), _| {
                        a.selected_mod = a.modlist.iter().position(|x| x == m)
                    }),
            )
            .padding(LABEL_SPACING)
    }))
    .vertical()
    .on_click(|_, data: &mut AppData, _| {
        // TODO: This is called when a label is clicked,
        // but luckily that occurs after this.
        data.selected_mod = None;
    })
    .border(Color::WHITE, 1.0)
    .expand()
    .background(BG_COLOR);

    // build information panel for selected mod
    let modinfo_panel = Label::new(|data: &AppData, _env: &Env| {
        if let Some(index) = data.selected_mod {
            if let Some(m) = data.modlist.get(index) {
                return format! {"Name: {}\nAuthor: {}\n\n{}", m.name, m.author, m.description};
            }
        }
        data.response.to_owned()
    })
    .with_line_break_mode(LineBreaking::WordWrap)
    .expand()
    .padding(LABEL_SPACING)
    .border(Color::WHITE, 1.0)
    .background(BG_COLOR);

    // Patch button
    let patch_button = Button::new(LocalizedString::new("Patch XBE"))
        .on_click(patch_button_on_click)
        .expand_width();

    // Arrange panels
    Flex::row()
        .with_flex_child(modlist_panel, 3.0)
        .with_spacer(PANEL_SPACING)
        .with_flex_child(
            Flex::column()
                .with_flex_child(modinfo_panel, 1.0)
                .with_spacer(PANEL_SPACING)
                .with_child(patch_button),
            2.0,
        )
        .padding(PANEL_SPACING)
}

fn set_response(data: &mut AppData, response: impl Into<String>) {
    let response = response.into();
    println!("RESPONSE: {}", response);

    data.selected_mod = None;
    data.response = response;
}

fn patch_button_on_click(ctx: &mut EventCtx, data: &mut AppData, _: &Env) {
    if !std::path::Path::new(data::PATH_ROM).is_file() {
        let types = vec![FileSpec::new("Xbox Executable", &["xbe"])];
        let options = FileDialogOptions::new()
            .allowed_types(types)
            .button_text("Import")
            .title("Import Clean ROM");

        ctx.submit_command(Command::new(
            druid::commands::SHOW_OPEN_PANEL,
            options,
            Target::Auto,
        ));
        return;
    }

    apply_enabled_mods(data);
}

fn apply_enabled_mods(data: &mut AppData) {
    let modlist = &data.modlist;
    let enabled_mods = data
        .enabled_mods
        .iter()
        .enumerate()
        .filter(|(_, enabled)| **enabled)
        .map(|(i, _)| &modlist[i])
        .collect::<Vec<&Mod>>();

    if enabled_mods.is_empty() {
        set_response(data, "No mods selected");
        return;
    }

    match Rom::new() {
        Err(e) => set_response(data, e.to_string()),
        Ok(mut rom) => {
            for m in enabled_mods {
                // Download Patch
                match m.download() {
                    Err(_) => {
                        let response = format!("Failed to download {}", m.name);
                        set_response(data, response);
                        return;
                    }
                    Ok(patch_bytes) => {
                        let mut patch = Patch::new(patch_bytes);
                        if patch.apply_to(&mut rom).is_err() {
                            let response = format!("Failed to apply {}", m.name);
                            set_response(data, response);
                            return;
                        }
                    }
                }
            }

            // Write out modified rom
            match rom.export() {
                Err(_) => set_response(data, "Failed to export patched rom!"),
                Ok(_) => set_response(data, "Successfully patched ROM."),
            }
        }
    }
}

pub struct Delegate;

impl AppDelegate<AppData> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppData,
        _env: &Env,
    ) -> Handled {
        if let Some(file_info) = cmd.get(druid::commands::OPEN_FILE) {
            if std::fs::create_dir_all("baserom").is_err() {
                set_response(data, "Failed to make baserom directory");
                return Handled::Yes;
            }
            if std::fs::copy(file_info.path(), data::PATH_ROM).is_err() {
                set_response(data, "Failed to copy rom");
                return Handled::Yes;
            }

            if let Ok(bytes) = std::fs::read(data::PATH_ROM) {
                if !Rom::verify_hash(&bytes) {
                    set_response(data, "The imported file is not correct.");
                    let _ = std::fs::remove_file(PATH_ROM);
                } else {
                    apply_enabled_mods(data);
                }
            }
            return Handled::Yes;
        }

        Handled::No
    }
}
