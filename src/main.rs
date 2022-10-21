use dataset::Class;
use dataset::Dataset;
use dataset::MyLabel;
use eframe::egui;
use egui::*;
use std::collections::HashMap;

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1920.0, 1080.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Show an image with eframe/egui",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

mod dataset {
    use crate::egui::*;
    use glob::glob;
    use image::ImageError;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::PathBuf;

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

    impl YoloLabel {
        // TODO implement FromStr and AsStr I guess
        fn as_string(self) -> String {
            format!(
                "{} {} {} {} {}",
                self.class_num, self.x, self.y, self.w, self.h
            )
        }
        fn from_str(yolo_str: &str) -> Self {
            let parts: Vec<_> = yolo_str.split(' ').collect();
            let class_num: usize = parts[0].parse().unwrap();
            let (x, y) = (parts[1].parse().unwrap(), parts[2].parse().unwrap());
            let (w, h) = (parts[3].parse().unwrap(), parts[4].parse().unwrap());
            Self {
                class_num,
                x,
                y,
                w,
                h,
            }
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
        pub fn from_input_dir() -> Self {
            let mut data = vec![];
            let labels_dir = PathBuf::from("./input");
            let mut paths: Vec<_> = glob("./input/*.jpg").unwrap().map(|p| p.unwrap()).collect();
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

            Dataset {
                labels_dir,
                data,
                i: first_no_label,
                current_labels: vec![],
            }
        }

        pub fn current_image(&mut self) -> Result<ColorImage, ImageError> {
            let path = &self.data[self.i].img_src;
            let (image, size) = load_image_from_path(path)?;
            let image_size = Vec2::from([size[0] as f32, size[1] as f32]);
            self.load_labels(image_size);
            Ok(image)
        }

        fn load_labels(&mut self, img_size: Vec2) {
            let img_filename = self.data[self.i].img_src.file_name().unwrap();
            let mut label_path = self.labels_dir.clone();
            label_path.push(img_filename);
            label_path.set_extension("txt");
            let yolo_strs = match std::fs::read_to_string(&label_path) {
                Ok(yolo_labels) => yolo_labels,
                Err(_) => return,
            };

            self.current_labels = yolo_strs
                .lines()
                .map(YoloLabel::from_str)
                .map(|yolo_label| (img_size, yolo_label).into())
                // TODO maybe remove
                // filter to clean up labels with wrong tiny boxes. Probably not needed in long term
                .filter(|label: &MyLabel| label.rect.width() > 5.0 && label.rect.height() > 5.0)
                .collect();
        }
        pub fn remove_labels(&mut self, pos: Pos2) {
            self.current_labels
                .retain(|label| !label.rect.contains(pos));
        }
        pub fn add_label(&mut self, label: MyLabel) {
            self.current_labels.push(label)
        }

        pub fn next(&mut self) {
            println!("Next image, now on: {}", self.i);
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
                println!("Saving label {label_path:?}");
                writeln!(file, "{}", yolo_label.as_string()).unwrap();
            }
        }
        pub fn previous(&mut self) {
            println!("Previous image, now on: {}", self.i);
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
    }
}

#[derive(Debug, Clone, Copy)]
enum BBoxInput {
    None,
    Partial(Pos2),
    Finished(Pos2, Pos2),
}

struct MyApp {
    texture: Option<egui::TextureHandle>,
    mask: Option<egui::TextureHandle>,
    bbox_input: BBoxInput,
    dataset: Dataset,
    current_class: Class,
    image_rect: Rect,
    filter: bool,
    shown_classes: HashMap<Class, bool>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            texture: None,
            mask: None,
            bbox_input: BBoxInput::None,
            dataset: Dataset::from_input_dir(),
            current_class: Class::V2,
            image_rect: Rect::NOTHING,
            filter: false,
            shown_classes: HashMap::new(),
        }
    }
}
impl MyApp {
    fn get_img_size(&self) -> Vec2 {
        match &self.texture {
            Some(t) => t.size_vec2(),
            None => unreachable!(),
        }
    }

    fn remove_bbs(&mut self, pos: Pos2) {
        self.dataset.remove_labels(pos);
    }

    fn to_img_coordinates(&self, pos: Pos2) -> Pos2 {
        (pos - self.image_rect.left_top()).to_pos2()
    }
    fn to_screen_coordinates(&self, pos: Pos2) -> Pos2 {
        pos + self.image_rect.left_top().to_vec2()
    }

    fn handle_img_response(&mut self, img_response: Response, ui: &mut Ui) {
        if img_response.secondary_clicked() {
            let screen_pos = img_response.interact_pointer_pos().unwrap();
            let pos = self.to_img_coordinates(screen_pos);
            self.remove_bbs(pos);
            self.update_mask(ui);
        }

        // secondary click also regiesters a drag, therefore early return
        if ui.input().pointer.button_down(PointerButton::Secondary) {
            return;
        }
        self.bbox_input = match self.bbox_input {
            BBoxInput::None if img_response.drag_started() => {
                let screen_pos = img_response.interact_pointer_pos().unwrap();
                let pos = self.to_img_coordinates(screen_pos);
                BBoxInput::Partial(pos)
            }
            BBoxInput::None => BBoxInput::None,
            BBoxInput::Partial(pos1) if img_response.drag_released() => {
                let screen_pos = img_response.interact_pointer_pos().unwrap();
                let pos2 = self.to_img_coordinates(screen_pos);
                // sometimes you drag a tiny amount without wanting to
                if (pos2.x - pos1.x).abs() < 5.0 || (pos2.y - pos1.y).abs() < 5.0 {
                    BBoxInput::Partial(pos1)
                } else {
                    BBoxInput::Finished(pos1, pos2)
                }
            }
            BBoxInput::Partial(pos1) => BBoxInput::Partial(pos1),
            BBoxInput::Finished(pos1, pos2) => {
                let class = self.current_class;
                let label = MyLabel {
                    class,
                    rect: Rect::from_two_pos(pos1, pos2),
                };
                println!("{:?}", label);
                self.dataset.add_label(label);
                self.update_mask(ui);
                BBoxInput::None
            }
        };
    }

    fn draw_label_text(&self, painter: &Painter, text_pos: Pos2, class: Class) {
        painter.rect(
            Rect::from_two_pos(text_pos, text_pos + [40.0, -35.0].into()),
            Rounding::none(),
            class.color(),
            Stroke::none(),
        );
        let _text_rect = painter.text(
            text_pos,
            Align2::LEFT_BOTTOM,
            class.to_name(),
            FontId::monospace(35.0),
            Color32::BLACK,
        );
    }

    fn draw_bbs(&self, ui: &mut Ui) {
        let painter = ui.painter();
        for label in &self.dataset.current_labels {
            let color = label.class.color();
            let screen_rect = [
                self.to_screen_coordinates(label.rect.left_top()),
                self.to_screen_coordinates(label.rect.right_bottom()),
            ]
            .into();
            painter.rect_stroke(screen_rect, Rounding::none(), Stroke::new(2.0, color));
            let text_pos = screen_rect.left_bottom();
            self.draw_label_text(painter, text_pos, label.class);
        }
    }

    fn draw_guide(&self, ui: &mut Ui, pos: Pos2) {
        let painter = ui.painter();
        let rect = ui.clip_rect();
        let w_size = rect.size();
        let color = self.current_class.color();
        let stroke = egui::Stroke::new(2.0, color);
        painter.hline(0.0..=w_size.x, pos.y, stroke);
        painter.vline(pos.x, 0.0..=w_size.y, stroke);
        self.draw_label_text(painter, pos, self.current_class);
    }

    fn draw_partial_box(&self, ui: &mut Ui) {
        if let BBoxInput::Partial(pos) = self.bbox_input {
            let screen_pos = self.to_screen_coordinates(pos);
            self.draw_guide(ui, screen_pos);
        }
    }

    fn class_pressed(&self, ctx: &Context) -> Option<Class> {
        for (key, class) in Class::shortcuts() {
            if ctx.input().key_pressed(key) {
                return Some(class);
            }
        }
        None
    }

    fn update_texture(&mut self, ui: &mut Ui) {
        let image = self.dataset.current_image().unwrap();
        let texture = ui
            .ctx()
            .load_texture("my-image", image, egui::TextureFilter::Linear);
        self.texture = Some(texture);
    }
    fn update_mask(&mut self, ui: &mut Ui) {
        let mask = self.dataset.generate_mask(&self.shown_classes);
        let texture = ui
            .ctx()
            .load_texture("mask", mask, egui::TextureFilter::Linear);
        self.mask = Some(texture);
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                // Draw image
                if self.texture.is_none() {
                    self.update_texture(ui);
                }
                let texture = self.texture.clone().unwrap();

                let img_response = ui.add(
                    egui::Image::new(&texture, texture.size_vec2()).sense(Sense::click_and_drag()),
                );
                self.image_rect = img_response.rect;

                // filter with mask
                if self.filter {
                    let mask: &TextureHandle = self.mask.get_or_insert_with(|| {
                        let mask = self.dataset.generate_mask(&self.shown_classes);
                        let texture =
                            ui.ctx()
                                .load_texture("mask", mask, egui::TextureFilter::Linear);
                        texture
                    });
                    ui.put(self.image_rect, egui::Image::new(mask, mask.size_vec2()));
                }

                // Handle clicks for bbs
                self.handle_img_response(img_response, ui);

                // Draw guides
                let pos = ctx.input().pointer.hover_pos();
                if let Some(pos) = pos {
                    self.draw_guide(ui, pos)
                }
                self.draw_partial_box(ui);

                // Draw bbs
                self.draw_bbs(ui);

                // Handle prev next picture keyboard
                let next_pressed = ctx.input().key_pressed(egui::Key::ArrowRight)
                    | ctx.input().key_pressed(egui::Key::D);
                if next_pressed {
                    self.dataset.save_labels(self.get_img_size());
                    self.dataset.next();
                    self.update_texture(ui);
                    self.update_mask(ui);
                }
                let previous_pressed = ctx.input().key_pressed(egui::Key::ArrowLeft)
                    | ctx.input().key_pressed(egui::Key::A);
                if previous_pressed {
                    // self.dataset.save_labels(self.get_img_size());
                    self.dataset.previous();
                    self.update_texture(ui);
                    self.update_mask(ui);
                }

                // Handle class setting
                // TODO refactor
                let class = self.class_pressed(ctx);
                if let Some(class) = class {
                    if self.filter {
                        let is_shown = self.shown_classes.entry(class).or_insert(false);
                        *is_shown = !*is_shown;
                        self.update_mask(ui);
                    } else {
                        self.current_class = class;
                    }
                }

                // Handle filter mode
                let filter_pressed = ctx.input().key_pressed(egui::Key::F);
                if filter_pressed {
                    self.filter = !self.filter;
                }
            });
    }
}
