#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use aho_corasick::AhoCorasick;
use eframe::egui;
use eframe::egui::TextBuffer;
use std::cmp::Ordering;
use std::ffi::{OsStr, OsString};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::{env, fs};

const N: usize = 12;
const STAT_SUM: u32 = 320;
const MAX_NAME_LEN: usize = 20;

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
    let mut args = env::args_os();
    let save_name = if let Some(name) = args.nth(1) {
        name
    } else {
        eprintln!("Please specify save file as parameter.");
        return;
    };
    let save_data = if let Ok(data) = fs::read(Path::new(&save_name)) {
        data
    } else {
        eprintln!("Failed to read the save file.");
        return;
    };

    let soldiers = parse_save(&save_data);
    let soldiers: [Soldier; N] = soldiers.try_into().unwrap();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 760.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Xenonauts save editor",
        options,
        Box::new(|_cc| {
            Box::new(MyApp {
                save_name,
                orig_save_data: save_data,
                soldiers,
                backup: true,
            })
        }),
    )
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

impl Soldier {
    fn sum(&self) -> u32 {
        self.tus + self.hps + self.str + self.acc + self.rfl + self.brv
    }
}

struct MyApp {
    save_name: OsString,
    orig_save_data: Vec<u8>,
    soldiers: [Soldier; N],
    backup: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("");
            ui.horizontal(|ui| {
                ui.label("    ");
                ui.heading("Xenonauts save editor");
                ui.label("        ");
                if ui.button("Save").clicked() {
                    write_save_file(
                        &self.save_name,
                        &write_save_data(&self.orig_save_data, &self.soldiers),
                        self.backup,
                    );
                }
                ui.label("    ");
                ui.checkbox(&mut self.backup, "Enable backup");
                ui.label("    ");
                ui.label(self.save_name.to_string_lossy());
            });

            let columns = 4;
            for i in 0..(N / columns) {
                ui.label("");
                ui.horizontal(|ui| {
                    for j in 0..columns {
                        let k = columns * i + j;
                        ui.vertical(|ui| {
                            let name_label = ui.label("Name: ");
                            ui.text_edit_singleline(&mut self.soldiers[k].name)
                                .labelled_by(name_label.id);
                            ui.add(
                                egui::Slider::new(&mut self.soldiers[k].tus, 35..=70).text("TUS"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.soldiers[k].hps, 35..=70).text("HPS"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.soldiers[k].str, 35..=70).text("STR"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.soldiers[k].acc, 35..=70).text("ACC"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.soldiers[k].rfl, 35..=70).text("RFL"),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.soldiers[k].brv, 35..=70).text("BRV"),
                            );
                            ui.label("");
                            let sum = self.soldiers[k].sum();
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

    let ac = AhoCorasick::new([MARK]);
    let ac2 = AhoCorasick::new([MARK2]);

    let mut matches = vec![];
    for mat in ac.find_iter(&save_data) {
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
