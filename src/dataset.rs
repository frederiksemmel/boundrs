use crate::egui::*;
use anyhow::{Error, Result};
use glob::glob;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    A = 0,
    K,
    Q,
    J,
    V10,
    V9,
    V8,
    V7,
    V6,
    V5,
    V4,
    V3,
    V2,
}

impl Class {
    pub fn color(self) -> Color32 {
        use Class::*;
        match self {
            A => Color32::from_rgb(0x2f, 0x4f, 0x4f),
            K => Color32::from_rgb(0x8b, 0x45, 0x13),
            Q => Color32::from_rgb(0x00, 0x80, 0x00),
            J => Color32::from_rgb(0x4b, 0x00, 0x82),
            V10 => Color32::from_rgb(0xff, 0x00, 0x00),
            V9 => Color32::from_rgb(0xff, 0xff, 0x00),
            V8 => Color32::from_rgb(0x00, 0xff, 0x00),
            V7 => Color32::from_rgb(0x00, 0xff, 0xff),
            V6 => Color32::from_rgb(0x00, 0x00, 0xff),
            V5 => Color32::from_rgb(0xff, 0x00, 0xff),
            V4 => Color32::from_rgb(0x64, 0x95, 0xed),
            V3 => Color32::from_rgb(0xff, 0xda, 0xb9),
            V2 => Color32::from_rgb(0xff, 0x69, 0xb6),
        }
    }

    pub fn shortcuts() -> HashMap<Key, Class> {
        let mut map = HashMap::new();
        map.insert(Key::Num1, Class::A);
        map.insert(Key::Num2, Class::V2);
        map.insert(Key::Num3, Class::V3);
        map.insert(Key::Num4, Class::V4);
        map.insert(Key::Num5, Class::V5);
        map.insert(Key::Num6, Class::V6);
        map.insert(Key::Num7, Class::V7);
        map.insert(Key::Num8, Class::V8);
        map.insert(Key::Num9, Class::V9);
        map.insert(Key::Num0, Class::V10);
        map.insert(Key::J, Class::J);
        map.insert(Key::Q, Class::Q);
        map.insert(Key::K, Class::K);
        map
    }

    fn from_usize(i: usize) -> Class {
        use Class::*;
        match i {
            0 => A,
            1 => K,
            2 => Q,
            3 => J,
            4 => V10,
            5 => V9,
            6 => V8,
            7 => V7,
            8 => V6,
            9 => V5,
            10 => V4,
            11 => V3,
            12 => V2,
            _ => unreachable!(),
        }
    }

    pub fn to_name(self) -> &'static str {
        use Class::*;
        match self {
            A => "A",
            K => "K",
            Q => "Q",
            J => "J",
            V10 => "10",
            V9 => "9",
            V8 => "8",
            V7 => "7",
            V6 => "6",
            V5 => "5",
            V4 => "4",
            V3 => "3",
            V2 => "2",
        }
    }
}

pub type YoloLabel = Vec<YoloBB>;

#[derive(Debug, Clone, Copy)]
pub struct YoloBB {
    class_num: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl FromStr for YoloBB {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<_> = s.split(' ').collect();
        let class_num: usize = parts[0].parse()?;
        let (x, y) = (parts[1].parse()?, parts[2].parse()?);
        let (w, h) = (parts[3].parse()?, parts[4].parse()?);
        Ok(Self {
            class_num,
            x,
            y,
            w,
            h,
        })
    }
}

impl YoloBB {
    fn as_string(self) -> String {
        format!(
            "{} {} {} {} {}",
            self.class_num, self.x, self.y, self.w, self.h
        )
    }
}

pub trait BoundingBox {
    fn rect(&self, size: Vec2) -> Rect;
    fn class(&self) -> Class;
    fn from_rect(rect: Rect, size: Vec2, class: Class) -> Self;
}

impl BoundingBox for YoloBB {
    fn rect(&self, size: Vec2) -> Rect {
        let img_w = size.x as f32;
        let img_h = size.y as f32;
        let yl = self;
        Rect::from_center_size(
            [yl.x * img_w, yl.y * img_h].into(),
            [yl.w * img_w, yl.h * img_h].into(),
        )
    }
    fn class(&self) -> Class {
        Class::from_usize(self.class_num)
    }
    fn from_rect(rect: Rect, size: Vec2, class: Class) -> Self {
        let img_w = size.x as f32;
        let img_h = size.y as f32;
        let center = rect.center();
        let x = center.x as f32 / img_w;
        let y = center.y as f32 / img_h;
        let size = rect.size();
        let w = size.x as f32 / img_w;
        let h = size.y as f32 / img_h;
        let class_num = class as usize;
        YoloBB {
            class_num,
            x,
            y,
            w,
            h,
        }
    }
}

#[derive(Debug)]
struct Datapoint {
    img_src: PathBuf,
    label_src: PathBuf,
}

fn load_image_from_path(path: &std::path::Path) -> Result<ColorImage> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}

impl Datapoint {
    fn new(img_src: PathBuf, labels_dir: PathBuf) -> Self {
        let img_filename = img_src.file_name().unwrap();
        let mut label_src = labels_dir;
        label_src.push(img_filename);
        label_src.set_extension("txt");
        Datapoint { img_src, label_src }
    }
    fn load_image(&self) -> Result<ColorImage> {
        load_image_from_path(&self.img_src)
    }
    fn load_label(&self) -> Result<YoloLabel> {
        if !self.label_src.exists() {
            File::create(&self.label_src).unwrap();
        }
        let yolo_strs = std::fs::read_to_string(&self.label_src)?;

        let mut labels = vec![];
        for line in yolo_strs.lines() {
            let label = YoloBB::from_str(line)?;
            labels.push(label)
        }
        Ok(labels)
    }
    fn save_label(&self, label: YoloLabel) -> Result<()> {
        let mut file = File::create(&self.label_src)?;
        for yolo_label in label {
            writeln!(file, "{}", yolo_label.as_string())?;
        }
        println!("Saving labels to {:?}", self.label_src);
        Ok(())
    }
    fn name(&self) -> String {
        self.img_src.file_name().unwrap().to_str().unwrap().into()
    }
}

pub enum DatasetMovement<'c> {
    Next,
    Previous,
    NextContaining(&'c HashSet<Class>),
    PreviousContaining(&'c HashSet<Class>),
}

pub struct Dataset {
    data: Vec<Datapoint>,
    i: usize,
}

impl Dataset {
    pub fn from_input_dir() -> Result<Self> {
        let mut data = vec![];
        let labels_dir = PathBuf::from("./input");
        let mut paths: Vec<_> = glob("./input/*.jpg")?.map(|p| p.unwrap()).collect();
        alphanumeric_sort::sort_path_slice(&mut paths);
        for img_src in paths.into_iter() {
            data.push(Datapoint::new(img_src, labels_dir.clone()))
        }
        // start at first imgage without labels
        let first_no_label = data
            .iter()
            .position(|p| !p.label_src.is_file())
            .unwrap_or(0);
        println!(
            "Starting at index {first_no_label} with label {:?}",
            data[first_no_label]
        );

        Ok(Dataset {
            data,
            i: first_no_label,
        })
    }

    pub fn current_image(&self) -> Result<ColorImage> {
        self.data[self.i].load_image()
    }
    pub fn current_label(&self) -> Result<YoloLabel> {
        self.data[self.i].load_label()
    }
    pub fn previous_label(&self) -> Result<YoloLabel> {
        let previous = self.i.saturating_sub(1);
        self.data[previous].load_label()
    }
    pub fn current_name(&self) -> String {
        self.data[self.i].name()
    }
    pub fn get_progress(&self) -> (usize, usize, usize) {
        (0, self.i, self.data.len())
    }
    fn save_label(&self, label: YoloLabel) -> Result<()> {
        self.data[self.i].save_label(label)
    }
    fn next(&mut self) -> Result<()> {
        self.i = std::cmp::min(self.i + 1, self.data.len() - 1);
        Ok(())
    }
    fn previous(&mut self) -> Result<()> {
        self.i = self.i.saturating_sub(1);
        Ok(())
    }
    fn next_containing(&mut self, classes: &HashSet<Class>) -> Result<()> {
        while self.i < self.data.len() - 1 {
            self.i += 1;
            let label = self.data[self.i].load_label()?;
            if label.iter().any(|bb| classes.contains(&bb.class())) {
                break;
            }
        }
        Ok(())
    }
    fn previous_containing(&mut self, classes: &HashSet<Class>) -> Result<()> {
        while self.i > 0 {
            self.i -= 1;
            let label = self.data[self.i].load_label()?;
            if label.iter().any(|bb| classes.contains(&bb.class())) {
                break;
            }
        }
        Ok(())
    }

    pub fn go(&mut self, movement: DatasetMovement, label: YoloLabel) -> Result<()> {
        self.save_label(label)?;
        match movement {
            DatasetMovement::Next => self.next(),
            DatasetMovement::Previous => self.previous(),
            DatasetMovement::NextContaining(classes) => self.next_containing(classes),
            DatasetMovement::PreviousContaining(classes) => self.previous_containing(classes),
        }
    }
}
