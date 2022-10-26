use crate::egui::*;
use anyhow::{Error, Result};
use glob::glob;
use std::collections::HashMap;
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

#[derive(Debug, Clone, Copy)]
struct YoloLabel {
    class_num: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl FromStr for YoloLabel {
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

impl YoloLabel {
    fn as_string(self) -> String {
        format!(
            "{} {} {} {} {}",
            self.class_num, self.x, self.y, self.w, self.h
        )
    }
}

impl From<(Vec2, MyLabel)> for YoloLabel {
    fn from(size_label: (Vec2, MyLabel)) -> Self {
        let img_size = size_label.0;
        let label = size_label.1;
        let img_w = img_size.x as f32;
        let img_h = img_size.y as f32;
        let center = label.rect.center();
        let x = center.x as f32 / img_w;
        let y = center.y as f32 / img_h;
        let size = label.rect.size();
        let w = size.x as f32 / img_w;
        let h = size.y as f32 / img_h;
        let class_num = label.class as usize;
        YoloLabel {
            class_num,
            x,
            y,
            w,
            h,
        }
    }
}

impl From<(Vec2, YoloLabel)> for MyLabel {
    fn from(size_label: (Vec2, YoloLabel)) -> Self {
        let img_size = size_label.0;
        let img_w = img_size.x as f32;
        let img_h = img_size.y as f32;
        let yl = size_label.1;
        let rect = Rect::from_center_size(
            [yl.x * img_w, yl.y * img_h].into(),
            [yl.w * img_w, yl.h * img_h].into(),
        );
        let class = Class::from_usize(yl.class_num);
        Self { class, rect }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MyLabel {
    pub class: Class,
    pub rect: Rect,
}

#[derive(Debug)]
struct Datapoint {
    img_src: std::path::PathBuf,
}

#[derive(Debug)]
pub struct Dataset {
    labels_dir: PathBuf,
    data: Vec<Datapoint>,
    i: usize,
    pub current_labels: Vec<MyLabel>,
}

use image::{Rgba, RgbaImage};

fn load_image_from_path(
    path: &std::path::Path,
) -> Result<(ColorImage, [usize; 2]), image::ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok((
        ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
        size,
    ))
}

impl Dataset {
    pub fn from_input_dir() -> Result<Self> {
        let mut data = vec![];
        let labels_dir = PathBuf::from("./input");
        let mut paths: Vec<_> = glob("./input/*.jpg")?.map(|p| p.unwrap()).collect();
        alphanumeric_sort::sort_path_slice(&mut paths);
        for img_src in paths.iter() {
            data.push(Datapoint {
                img_src: img_src.clone(),
            })
        }
        // start at first imgage without labels
        let mut first_no_label = 0;
        for (i, p) in paths.iter().enumerate() {
            let mut label_path = labels_dir.clone();
            label_path.push(p.file_name().unwrap());
            label_path.set_extension("txt");
            if !label_path.is_file() {
                first_no_label = i;
                break;
            }
        }
        println!(
            "Starting at index {first_no_label} with img_src {:?}",
            data[first_no_label]
        );

        Ok(Dataset {
            labels_dir,
            data,
            i: first_no_label,
            current_labels: vec![],
        })
    }

    pub fn current_image(&mut self) -> Result<ColorImage> {
        let path = &self.data[self.i].img_src;
        let (image, size) = load_image_from_path(path)?;
        let image_size = Vec2::from([size[0] as f32, size[1] as f32]);
        let yolo_labels = self.load_labels(self.i)?;
        // TODO maybe move all stuff related to image size to app / ui code
        // then dataset would only know YoloLabel, cleaner
        self.current_labels = yolo_labels
            .into_iter()
            .map(|l| (image_size, l).into())
            .collect();
        Ok(image)
    }

    fn load_labels(&mut self, i: usize) -> Result<Vec<YoloLabel>> {
        let img_filename = self.data[i].img_src.file_name().unwrap();
        let mut label_path = self.labels_dir.clone();
        label_path.push(img_filename);
        label_path.set_extension("txt");
        let yolo_strs = std::fs::read_to_string(&label_path)?;

        let mut labels = vec![];
        for line in yolo_strs.lines() {
            let label = YoloLabel::from_str(line)?;
            labels.push(label)
        }
        Ok(labels)
    }

    fn get_filename(&self, i: usize) -> PathBuf {
        self.data[i].img_src.clone()
    }
    pub fn get_current_filename(&self) -> PathBuf {
        self.get_filename(self.i)
    }
    pub fn get_progress(&self) -> (usize, usize, usize) {
        (0, self.i, self.data.len())
    }
    pub fn remove_labels(&mut self, pos: Pos2) {
        self.current_labels
            .retain(|label| !label.rect.contains(pos));
    }
    pub fn add_label(&mut self, label: MyLabel) {
        self.current_labels.push(label)
    }

    pub fn next(&mut self) {
        self.i = std::cmp::min(self.i + 1, self.data.len() - 1);
    }
    pub fn save_labels(&mut self, img_size: Vec2) {
        // self.data[self.i].labels = self.current_labels.clone();
        // Save to file
        // get label filepath
        let mut label_path = self.data[self.i].img_src.clone();
        label_path.set_extension("txt");
        let mut file = File::create(&label_path).unwrap();
        for label in &self.current_labels {
            let yolo_label: YoloLabel = (img_size, *label).into();
            writeln!(file, "{}", yolo_label.as_string()).unwrap();
        }
        println!("Saving labels to {label_path:?}");
    }
    pub fn previous(&mut self) {
        self.i = self.i.saturating_sub(1);
    }
    fn pos_inside_label_box(&self, pos: Pos2, shown_classes: &HashMap<Class, bool>) -> bool {
        self.current_labels
            .iter()
            .any(|l| l.rect.contains(pos) && *shown_classes.get(&l.class).unwrap_or(&false))
    }
    pub fn generate_mask(&self, shown_classes: &HashMap<Class, bool>) -> ColorImage {
        let size: [usize; 2] = [1920, 1080];
        let mask = RgbaImage::from_fn(1920, 1080, |x, y| {
            let pos = Pos2::new(x as f32, y as f32);
            if self.pos_inside_label_box(pos, shown_classes) {
                Rgba([0, 0, 0, 0])
            } else {
                Rgba([0, 0, 0, 253])
            }
        });
        // let image_buffer = mask.to_rgba8();
        let pixels = mask.as_flat_samples();
        ColorImage::from_rgba_unmultiplied(size, pixels.as_slice())
    }

    pub fn repeat_bbs(&mut self, img_size: Vec2) -> Result<()> {
        let yolo_labels = self.load_labels(usize::saturating_sub(self.i, 1))?;
        self.current_labels = yolo_labels
            .into_iter()
            .map(|l| (img_size, l).into())
            .collect();
        Ok(())
    }
}
