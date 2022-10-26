use eframe::egui;
use egui::*;
use std::collections::HashMap;

mod dataset;
use crate::dataset::Class;
use dataset::Dataset;
use dataset::MyLabel;

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
            dataset: Dataset::from_input_dir().unwrap(),
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
            self.update_mask(ui.ctx());
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
                self.update_mask(ui.ctx());
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

    fn update_texture(&mut self, ctx: &Context) {
        let image = self.dataset.current_image().unwrap();
        let texture = ctx.load_texture("my-image", image, egui::TextureFilter::Linear);
        self.texture = Some(texture);
    }
    fn update_mask(&mut self, ctx: &Context) {
        let mask = self.dataset.generate_mask(&self.shown_classes);
        let texture = ctx.load_texture("mask", mask, egui::TextureFilter::Linear);
        self.mask = Some(texture);
    }

    fn handle_class_keys(&mut self, ctx: &Context) {
        let class = self.class_pressed(ctx);
        if let Some(class) = class {
            if self.filter {
                let is_shown = self.shown_classes.entry(class).or_insert(false);
                *is_shown = !*is_shown;
                self.update_mask(ctx);
            } else {
                self.current_class = class;
            }
        }
    }

    fn handle_left_right(&mut self, ctx: &Context) {
        let next_pressed =
            ctx.input().key_pressed(egui::Key::ArrowRight) | ctx.input().key_pressed(egui::Key::D);
        if next_pressed {
            self.dataset.save_labels(self.get_img_size());
            self.dataset.next();
            self.update_texture(ctx);
            self.update_mask(ctx);
        }
        let previous_pressed =
            ctx.input().key_pressed(egui::Key::ArrowLeft) | ctx.input().key_pressed(egui::Key::A);
        if previous_pressed {
            // self.dataset.save_labels(self.get_img_size());
            self.dataset.previous();
            self.update_texture(ctx);
            self.update_mask(ctx);
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("Boundrs Labeling").show(ctx, |ui| {
            let filename = self.dataset.get_current_filename();
            let filename = filename.to_str().unwrap_or("None");
            ui.label(format!("Current img source: {}", filename));

            let (_, current, max) = self.dataset.get_progress();
            ui.add(
                ProgressBar::new(current as f32 / max as f32)
                    .show_percentage()
                    .text(format!("{current} out of {max} images")),
            );
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                // Draw image
                if self.texture.is_none() {
                    self.update_texture(ctx);
                }
                let texture = self.texture.clone().unwrap();

                let img_response = ui.add(
                    egui::Image::new(&texture, texture.size_vec2()).sense(Sense::click_and_drag()),
                );
                self.image_rect = img_response.rect;

                // filter with mask
                // TODO find better way to do this
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

                // Draw guides
                let pos = ctx.input().pointer.hover_pos();
                if let Some(pos) = pos {
                    self.draw_guide(ui, pos)
                }
                self.draw_partial_box(ui);

                // Draw bbs
                self.draw_bbs(ui);

                // Draw info window

                // Handle clicks for bbs
                self.handle_img_response(img_response, ui);

                // Handle prev next picture keyboard
                self.handle_left_right(ctx);

                // Handle class setting
                self.handle_class_keys(ctx);

                // Handle filter mode
                let filter_pressed = ctx.input().key_pressed(egui::Key::F);
                if filter_pressed {
                    self.filter = !self.filter;
                }

                // Handle repeat button
                if ctx.input().key_pressed(egui::Key::R) {
                    self.dataset.repeat_bbs(self.get_img_size()).unwrap();
                }
            });
    }
}
