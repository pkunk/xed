#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use aho_corasick::AhoCorasick;
use eframe::egui;
use eframe::egui::TextBuffer;
use egui_file::FileDialog;
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

const N: usize = 12;
const MAX_NAME_LEN: usize = 20;

const STAT_SUM: u32 = 320;
const MIN_STAT: u32 = 35;
const MAX_STAT: u32 = 70;

// MARK....Soldier
static MARK: &[u8] = &[
    0x4d, 0x41, 0x52, 0x4b, 0x07, 0x00, 0x00, 0x00, 0x53, 0x6f, 0x6c, 0x64, 0x69, 0x65, 0x72,
];

// MARK....Soldier2
static MARK2: &[u8] = &[
    0x4d, 0x41, 0x52, 0x4b, 0x08, 0x00, 0x00, 0x00, 0x53, 0x6f, 0x6c, 0x64, 0x69, 0x65, 0x72, 0x32,
];

// Standard separator
static SEP: &[u8] = &[0x00, 0x00, 0x00];

fn main() {
    let mut soldiers = Vec::new();
    for _ in 0..N {
        soldiers.push(Soldier::default());
    }
    let soldiers: [Soldier; N] = soldiers.try_into().unwrap();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(680.0, 400.0)),
        ..Default::default()
    };
    let gui = eframe::run_native(
        "Xenonauts CE save editor",
        options,
        Box::new(|_cc| {
            Box::new(MyApp {
                save_name: None,
                orig_save_data: None,
                soldiers,
                backup: true,
                open_file_dialog: None,
            })
        }),
    );
    if gui.is_err() {
        eprintln!("Failed to run GUI.");
    }
}

#[derive(Clone, Debug)]
struct NameString {
    text: String,
}

#[derive(Clone, Debug)]
struct Soldier {
    name: NameString,
    tus: u32,
    hps: u32,
    str: u32,
    acc: u32,
    rfl: u32,
    brv: u32,
    orig_name_offset: usize,
    orig_name_len: usize,
    orig_stats_offset: usize,
}

impl Default for Soldier {
    fn default() -> Self {
        Soldier {
            name: NameString {
                text: "None".to_owned(),
            },
            tus: MIN_STAT,
            hps: MIN_STAT,
            str: MIN_STAT,
            acc: MIN_STAT,
            rfl: MIN_STAT,
            brv: MIN_STAT,
            orig_name_offset: 0,
            orig_name_len: 0,
            orig_stats_offset: 0,
        }
    }
}

impl Soldier {
    fn sum(&self) -> u32 {
        self.tus + self.hps + self.str + self.acc + self.rfl + self.brv
    }
}

struct MyApp {
    save_name: Option<OsString>,
    orig_save_data: Option<Vec<u8>>,
    soldiers: [Soldier; N],
    backup: bool,
    open_file_dialog: Option<FileDialog>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("");
            ui.horizontal(|ui| {
                ui.label("        ");
                if ui.button("Open").clicked() {
                    let mut dialog = FileDialog::open_file(None);
                    dialog.open();
                    self.open_file_dialog = Some(dialog);
                }
                if let Some(dialog) = &mut self.open_file_dialog {
                    if dialog.show(ctx).selected() {
                        if let Some(file) = dialog.path() {
                            let save_name = file.try_into().unwrap();
                            let save_data = if let Ok(data) = fs::read(Path::new(&save_name)) {
                                data
                            } else {
                                eprintln!("Failed to read the save file.");
                                return;
                            };
                            let soldiers = parse_save(&save_data);
                            if soldiers.len() != N {
                                eprintln!("Invalid save file.");
                                return;
                            }
                            let soldiers: [Soldier; N] = soldiers.try_into().unwrap();
                            self.save_name = Some(save_name);
                            self.orig_save_data = Some(save_data);
                            self.soldiers = soldiers;
                        }
                    }
                }
                if ui.button("Save").clicked() {
                    if let Some(save_name) = &self.save_name {
                        write_save_file(
                            save_name,
                            &write_save_data(self.orig_save_data.as_ref().unwrap(), &self.soldiers),
                            self.backup,
                        );
                    }
                }
                ui.label("    ");
                ui.checkbox(&mut self.backup, "Enable backup");
                ui.label("    ");
                if let Some(save_name) = &self.save_name {
                    ui.label(save_name.to_string_lossy());
                }
            });

            ui.label("");

            ui.horizontal(|ui| {
                ui.label("                                                                                                   ");
                ui.label("TUS      ");
                ui.label("HPS      ");
                ui.label("STR      ");
                ui.label("ACC      ");
                ui.label("RFL      ");
                ui.label("BRV      ");
            });

            for i in 0..N {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.soldiers[i].name);
                    ui.add(
                        egui::DragValue::new(&mut self.soldiers[i].tus)
                            .clamp_range(MIN_STAT..=MAX_STAT)
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.soldiers[i].hps)
                            .clamp_range(MIN_STAT..=MAX_STAT),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.soldiers[i].str)
                            .clamp_range(MIN_STAT..=MAX_STAT),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.soldiers[i].acc)
                            .clamp_range(MIN_STAT..=MAX_STAT),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.soldiers[i].rfl)
                            .clamp_range(MIN_STAT..=MAX_STAT),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.soldiers[i].brv)
                            .clamp_range(MIN_STAT..=MAX_STAT),
                    );
                    ui.label("");
                    let sum = self.soldiers[i].sum();
                    let sum_text = format!("SUM: {sum}");
                    match sum.cmp(&STAT_SUM) {
                        Ordering::Greater => ui.label(sum_text),
                        Ordering::Less => ui.weak(sum_text),
                        Ordering::Equal => ui.strong(sum_text),
                    }
                });
            }
        });
    }
}

impl TextBuffer for NameString {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.text.as_ref()
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        if !text.is_ascii() || text.len() + self.text.len() > 20 {
            return 0;
        }

        String::insert_text(&mut self.text, text, char_index)
    }

    fn delete_char_range(&mut self, char_range: Range<usize>) {
        String::delete_char_range(&mut self.text, char_range);
    }

    fn clear(&mut self) {
        self.text.clear();
    }

    fn replace(&mut self, text: &str) {
        if text.is_ascii() && text.len() <= MAX_NAME_LEN {
            self.text = text.to_owned();
        }
    }

    fn take(&mut self) -> String {
        std::mem::take(&mut self.text)
    }
}

fn parse_save(save_data: &[u8]) -> Vec<Soldier> {
    let mut soldiers = vec![];

    let ac = AhoCorasick::new([MARK]).unwrap();
    let ac2 = AhoCorasick::new([MARK2]).unwrap();

    let mut matches = vec![];
    for mat in ac.find_iter(save_data) {
        let start = mat.end() + 1;
        let end = ac2.find_iter(&save_data[start..]).next().unwrap().start() + start;
        matches.push((start, end));
    }

    for (start, _end) in matches {
        let mut cursor = start + SEP.len();

        let next_size = save_data[cursor] as usize;
        cursor += SEP.len() + 1;
        let nationality = String::from_utf8_lossy(&save_data[cursor..cursor + next_size]);
        cursor += nationality.len();

        let next_size = save_data[cursor] as usize;
        let orig_name_offset = cursor;
        let orig_name_len = next_size;
        cursor += SEP.len() + 1;
        let name = String::from_utf8_lossy(&save_data[cursor..cursor + next_size]);
        cursor += name.len();

        let next_size = save_data[cursor] as usize;
        cursor += SEP.len() + 1;
        let portrait = String::from_utf8_lossy(&save_data[cursor..cursor + next_size]);
        cursor += portrait.len();

        cursor += SEP.len() + 1;

        let next_size = save_data[cursor] as usize;
        cursor += SEP.len() + 1;
        let country = String::from_utf8_lossy(&save_data[cursor..cursor + next_size]);
        cursor += country.len();

        let orig_stats_offset = cursor;

        cursor += SEP.len() + 1;
        let hps = &save_data[cursor];

        cursor += SEP.len() + 1;
        let str = &save_data[cursor];

        cursor += SEP.len() + 1;
        let acc = &save_data[cursor];

        cursor += SEP.len() + 1;
        let rfl = &save_data[cursor];

        cursor += SEP.len() + 1;
        let brv = &save_data[cursor];

        cursor += SEP.len() + 1;
        let tus = &save_data[cursor];

        let soldier = Soldier {
            name: NameString {
                text: name.to_string(),
            },
            tus: (*tus) as u32,
            hps: (*hps) as u32,
            str: (*str) as u32,
            acc: (*acc) as u32,
            rfl: (*rfl) as u32,
            brv: (*brv) as u32,
            orig_name_offset,
            orig_name_len,
            orig_stats_offset,
        };
        if soldiers.len() < N {
            soldiers.push(soldier);
        }
    }
    soldiers
}

fn write_save_data(orig_data: &[u8], soldiers: &[Soldier]) -> Vec<u8> {
    let mut result = vec![];
    result.extend_from_slice(orig_data);
    for s in soldiers.iter().rev() {
        let mut cursor = s.orig_stats_offset;

        cursor += SEP.len() + 1;
        result[cursor] = s.hps as u8;
        cursor += SEP.len() + 1;
        result[cursor] = s.str as u8;
        cursor += SEP.len() + 1;
        result[cursor] = s.acc as u8;
        cursor += SEP.len() + 1;
        result[cursor] = s.rfl as u8;
        cursor += SEP.len() + 1;
        result[cursor] = s.brv as u8;
        cursor += SEP.len() + 1;
        result[cursor] = s.tus as u8;

        let offset = s.orig_name_offset + 1 + SEP.len();
        if s.name.text.len() == s.orig_name_len {
            result[offset..(offset + s.orig_name_len)].copy_from_slice(s.name.text.as_bytes());
        } else {
            result[s.orig_name_offset] = s.name.text.len() as u8;
            let bytes = s.name.text.as_bytes().to_vec();
            let _: Vec<_> = result
                .splice(offset..(offset + s.orig_name_len), bytes)
                .collect();
        }
    }

    result
}

fn write_save_file(save_name: &OsStr, save_data: &[u8], backup: bool) {
    let path = Path::new(save_name);
    if !backup || !path.exists() {
        if fs::write(path, save_data).is_err() {
            eprintln!("Failed to write a save file.");
        }
        return;
    }

    let mut backup_path = PathBuf::from(save_name);
    backup_path.set_extension("bak");

    if backup_path.exists() {
        let mut found = false;
        for i in 0..256 {
            backup_path.set_extension(format!("bak{i}"));
            if !backup_path.exists() {
                found = true;
                break;
            }
        }
        if !found {
            eprintln!("Failed to create a backup file.");
            return;
        }
    }

    if fs::copy(path, backup_path).is_err() {
        eprintln!("Failed to write a backup file.");
        return;
    };

    if fs::write(path, save_data).is_err() {
        eprintln!("Failed to write a save file.");
    }
}
