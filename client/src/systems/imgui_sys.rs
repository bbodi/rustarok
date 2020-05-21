use crate::components::char::HasServerIdComponent;
use crate::components::controller::LocalPlayerController;
use crate::strum::IntoEnumIterator;
use crate::SIMULATION_FREQ;
use imgui::*;
use rustarok_common::common::{EngineTime, GameTime, Local};
use rustarok_common::components::char::{
    CharOutlook, CharState, CharType, EntityId, JobId, LocalCharStateComp, MonsterId,
    StaticCharDataComponent, Team,
};
use rustarok_common::components::job_ids::JobSpriteId;
use specs::prelude::{Join, WorldExt};
use specs::WriteStorage;
use std::collections::LinkedList;
use std::fmt::Write;

pub struct ImguiSys;

const PING_COUNT: usize = 16;
pub struct ImguiData {
    entity_under_cursor: Option<EntityId<Local>>,
    max_fps: usize,
    show_network_window: bool,
    show_entity_window: bool,
    pings: [f32; PING_COUNT],
    unacked_prediction_count: f32,
    rollbacks_per_second: [f32; PING_COUNT],
    fps: [f32; PING_COUNT],
    inc_packets_per_second: [f32; PING_COUNT],
    out_packets_per_second: [f32; PING_COUNT],
    incoming_bytes_per_second: [f32; PING_COUNT],
    outgoing_bytes_per_second: [f32; PING_COUNT],
    simulation_duration: f32,
    inspected_entities: Vec<EntityId<Local>>,
}

impl ImguiData {
    pub fn new(max_fps: usize) -> ImguiData {
        ImguiData {
            entity_under_cursor: None,
            max_fps,
            show_network_window: false,
            show_entity_window: false,
            pings: [0.0; PING_COUNT],
            fps: [0.0; PING_COUNT],
            inc_packets_per_second: [0.0; PING_COUNT],
            out_packets_per_second: [0.0; PING_COUNT],
            incoming_bytes_per_second: [0.0; PING_COUNT],
            outgoing_bytes_per_second: [0.0; PING_COUNT],
            rollbacks_per_second: [0.0; PING_COUNT],
            unacked_prediction_count: 0.0,
            simulation_duration: 0.0,
            inspected_entities: Vec::with_capacity(8),
        }
    }

    pub fn ping(&mut self, ping: usize) {
        self.pings.rotate_left(1);
        self.pings[PING_COUNT - 1] = ping as f32;
    }

    pub fn incoming_packets_per_second(&mut self, ping: usize) {
        self.inc_packets_per_second.rotate_left(1);
        self.inc_packets_per_second[PING_COUNT - 1] = ping as f32;
    }

    pub fn incoming_bytes_per_second(&mut self, ping: usize) {
        self.incoming_bytes_per_second.rotate_left(1);
        self.incoming_bytes_per_second[PING_COUNT - 1] = ping as f32;
    }

    pub fn outgoing_bytes_per_second(&mut self, ping: usize) {
        self.outgoing_bytes_per_second.rotate_left(1);
        self.outgoing_bytes_per_second[PING_COUNT - 1] = ping as f32;
    }

    pub fn outgoing_packets_per_second(&mut self, ping: usize) {
        self.out_packets_per_second.rotate_left(1);
        self.out_packets_per_second[PING_COUNT - 1] = ping as f32;
    }

    pub fn unacked_prediction_count(&mut self, ping: usize) {
        self.unacked_prediction_count = ping as f32;
    }

    pub fn rollback(&mut self, had_rollback: bool) {
        if had_rollback {
            self.rollbacks_per_second[PING_COUNT - 1] += 1.0;
        }
    }

    pub fn set_rollback(&mut self) {
        self.rollbacks_per_second.rotate_left(1);
        self.rollbacks_per_second[PING_COUNT - 1] = 0.0;
    }

    pub fn fps(&mut self, ping: usize) {
        self.fps.rotate_left(1);
        self.fps[PING_COUNT - 1] = ping as f32;
    }

    pub fn simulation_duration(&mut self, ping: usize) {
        self.simulation_duration = 1000.0 / (ping as f32);
    }

    pub fn inspect_entity(&mut self, entity: EntityId<Local>) {
        if self.inspected_entities.contains(&entity) {
            return;
        }
        self.inspected_entities.push(entity);
    }

    pub fn is_inspecting(&self, entity: EntityId<Local>) -> bool {
        self.inspected_entities.contains(&entity)
    }

    pub fn stop_inspecting_entity(&mut self, entity: EntityId<Local>) {
        let index = self
            .inspected_entities
            .iter()
            .position(|it| *it == entity)
            .unwrap();
        self.inspected_entities.swap_remove(index);
    }
}

pub fn draw_imgui(ecs_world: &mut specs::World, ui: &imgui::Ui) {
    Window::new(im_str!("###main_window"))
        .movable(false)
        .no_decoration()
        .always_auto_resize(true)
        .save_settings(false)
        .no_nav()
        .bg_alpha(0.75)
        .focus_on_appearing(true)
        .position([10.0, 10.0], Condition::FirstUseEver)
        .build(&ui, || {
            ui.set_window_font_scale(1.5);

            {
                if ui.is_window_hovered() && ui.is_mouse_clicked(MouseButton::Right) {
                    ui.open_popup(im_str!("###upper_right_context_menu"))
                }

                let data = &mut ecs_world.write_resource::<ImguiData>();
                let fps = data.fps[PING_COUNT - 1];
                let fps_color = color_gradient(
                    0.0,                 // start
                    data.max_fps as f32, // end
                    0.0,                 // start_offset
                    fps as f32,
                    [1.0, 0.0, 0.0, 1.0], // start color
                    [0.0, 1.0, 0.0, 1.0], // end color
                );
                ui.text_colored(fps_color, im_str!("FPS: {}", fps));

                let ping = data.pings[PING_COUNT - 1];
                let ping_color = color_gradient(
                    0.0,          // start
                    200.0 as f32, // end
                    20.0,         // start_offset
                    fps as f32,
                    [0.0, 1.0, 0.0, 1.0], // start color
                    [1.0, 0.0, 0.0, 1.0], // end color
                );
                ui.text_colored(ping_color, im_str!("Ping: {}", ping));

                ui.popup(im_str!("###upper_right_context_menu"), || {
                    if Selectable::new(im_str!("Network")).build(ui) {
                        data.show_network_window = !data.show_network_window;
                    }
                    if Selectable::new(im_str!("Entity")).build(ui) {
                        data.show_entity_window = !data.show_entity_window;
                    }
                });
            }
        });

    let (show_network_window, show_entity_window) = {
        let data = &mut ecs_world.write_resource::<ImguiData>();
        (data.show_network_window, data.show_entity_window)
    };

    if show_network_window {
        let data = &mut ecs_world.write_resource::<ImguiData>();
        draw_network_window(data, ui);
    }

    if show_entity_window {
        draw_entity_window(ecs_world, ui);
    }
    draw_inspected_entity_windows(ecs_world, ui);

    // general entity context menu
    if ui.io().key_ctrl && ui.io().key_shift {
        let entity_under_cursor = ecs_world
            .read_resource::<LocalPlayerController>()
            .entities_below_cursor
            .get_enemy_or_friend();
        ecs_world.write_resource::<ImguiData>().entity_under_cursor = entity_under_cursor;
        if entity_under_cursor.is_some() {
            ui.open_popup(im_str!("###entity_context_menu"))
        }
    }
    ui.popup(im_str!("###entity_context_menu"), || {
        let entity = ecs_world
            .write_resource::<ImguiData>()
            .entity_under_cursor
            .unwrap();
        if Selectable::new(im_str!("Inspect")).build(ui) {
            ecs_world
                .write_resource::<ImguiData>()
                .inspect_entity(entity);
        }
        if Selectable::new(im_str!("Etc")).build(ui) {}
    });
}

fn draw_entity_window(ecs_world: &mut specs::World, ui: &imgui::Ui) {
    let static_char_data_storage = ecs_world.read_storage::<StaticCharDataComponent>();
    let mut opened = ecs_world.read_resource::<ImguiData>().show_entity_window;
    Window::new(im_str!("Entities"))
        .opened(&mut opened)
        .size([300.0, 700.0], Condition::FirstUseEver)
        .build(&ui, || {
            ui.set_window_font_scale(1.5);

            if ui
                .collapsing_header(im_str!("Left"))
                .default_open(true)
                .build()
            {
                for (entity_id, static_char_data) in
                    (&ecs_world.entities(), &static_char_data_storage).join()
                {
                    if static_char_data.typ == CharType::Player
                        && static_char_data.team == Team::Left
                    {
                        player_info(entity_id, static_char_data, ecs_world, ui);
                    }
                }
            }
            if ui
                .collapsing_header(im_str!("Right"))
                .default_open(true)
                .build()
            {
                for (entity_id, static_char_data) in
                    (&ecs_world.entities(), &static_char_data_storage).join()
                {
                    if static_char_data.typ == CharType::Player
                        && static_char_data.team == Team::Right
                    {
                        player_info(entity_id, static_char_data, ecs_world, ui);
                    }
                }
            }
            if ui
                .collapsing_header(im_str!("Other"))
                .default_open(false)
                .build()
            {
                for (entity_id, static_char_data) in
                    (&ecs_world.entities(), &static_char_data_storage).join()
                {
                    if static_char_data.typ != CharType::Player {
                        player_info(entity_id, static_char_data, ecs_world, ui);
                    }
                }
            }
        });
    ecs_world.write_resource::<ImguiData>().show_entity_window = opened;
}

fn player_info(
    entity_id: specs::Entity,
    static_data: &StaticCharDataComponent,
    ecs_world: &specs::World,
    ui: &imgui::Ui,
) {
    let char_state_storage = ecs_world.read_storage::<LocalCharStateComp<Local>>();
    let char_state = char_state_storage.get(entity_id).unwrap();
    let hp_frac = char_state.hp as f32 / char_state.calculated_attribs().max_hp as f32;

    let mut progress_bar_text = String::with_capacity("PlayerName 99 999/99 999".len());
    progress_bar_text.write_str(&static_data.name);
    progress_bar_text.write_char(' ');

    write_into_str(char_state.hp, &mut progress_bar_text, 6);
    progress_bar_text.write_char('/');
    write_into_str(
        char_state.calculated_attribs().max_hp,
        &mut progress_bar_text,
        6,
    );

    let color = color_gradient(
        1.0, // start
        0.0, // end
        0.0, // start_offset
        hp_frac,
        [0.0, 1.0, 0.0, 1.0], // start color
        [1.0, 0.0, 0.0, 1.0], // end color
    );
    let color_token = ui.push_style_color(StyleColor::PlotHistogram, color.clone());
    ui.bullet();

    if ui.button(
        &im_str!(
            "{}###{}",
            static_data.name,
            EntityId::new(entity_id).as_u64()
        ),
        [0.0, 0.0],
    ) {
        ecs_world
            .write_resource::<ImguiData>()
            .inspect_entity(EntityId::new(entity_id));
    }
    ui.same_line(150.0);
    imgui::ProgressBar::new(hp_frac)
        .size([0.0, 0.0])
        .overlay_text(&ImString::new(progress_bar_text))
        .build(ui);
    color_token.pop(ui);
}

fn itoa(mut num: i32) -> ([u8; 14], usize) {
    let mut arr: [u8; 14] = [' ' as u8; 14];
    let mut i = 14;
    while num > 0 {
        i -= 1;
        if i == 10 || i == 6 || i == 2 {
            i -= 1;
        }
        arr[i] = ((num % 10) as u8 + '0' as u8);
        num = num / 10;
    }
    return (arr, i);
}

fn write_into_str(num: i32, dst: &mut String, len_with_padding: usize) {
    write_into_str2(itoa(num), dst, len_with_padding);
}

fn write_into_str2(a: ([u8; 14], usize), dst: &mut String, len_with_padding: usize) {
    let (array, original_start_index) = a;
    let len_without_padding = 14 - original_start_index;
    let len = len_with_padding.max(len_without_padding);
    let start = 14 - len;
    for i in start..14 {
        dst.write_char(array[i] as char);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Add;

    fn convert_to_str(a: ([u8; 14], usize), len_with_padding: usize) -> String {
        let (mut array, original_start_index) = a;
        let mut ret = String::with_capacity(14);
        write_into_str2(a, &mut ret, len_with_padding);
        ret
    }

    #[test]
    fn test_itoa() {
        assert_eq!(&convert_to_str(itoa(12), 0), "12");
        assert_eq!(&convert_to_str(itoa(123), 0), "123");
        assert_eq!(&convert_to_str(itoa(1234), 0), "1 234");
        assert_eq!(&convert_to_str(itoa(12345), 0), "12 345");
        assert_eq!(&convert_to_str(itoa(123456), 0), "123 456");
        assert_eq!(&convert_to_str(itoa(1234567), 0), "1 234 567");
        assert_eq!(&convert_to_str(itoa(12345678), 0), "12 345 678");
        assert_eq!(&convert_to_str(itoa(123456789), 0), "123 456 789");
    }
}

fn draw_inspected_entity_windows(ecs_world: &mut specs::World, ui: &imgui::Ui) {
    let mut stop_inspecting_entity_id = None;
    {
        let imgui_data = &mut ecs_world.read_resource::<ImguiData>();
        let mut static_char_data_storage =
            &mut ecs_world.write_storage::<StaticCharDataComponent>();
        let mut char_state_storage = &mut ecs_world.write_storage::<LocalCharStateComp<Local>>();
        let now = ecs_world.read_resource::<EngineTime>().now();
        let serer_id_storage = &ecs_world.read_storage::<HasServerIdComponent>();
        for inspected_entity_id in imgui_data.inspected_entities.iter() {
            let name = format!("{}", inspected_entity_id.as_u64());
            let mut opened = true;
            Window::new(&ImString::new(name))
                .size([300.0, 700.0], Condition::FirstUseEver)
                .opened(&mut opened)
                .build(&ui, || {
                    let second_column_x = 150.0;

                    ui.set_window_font_scale(1.3);
                    let server_id = serer_id_storage.get((*inspected_entity_id).into()).unwrap();
                    ui.text(im_str!("Server ID"));
                    ui.same_line(second_column_x);
                    ui.text(im_str!("{}", server_id.server_id.as_u64()));

                    entity_outlook(
                        ui,
                        inspected_entity_id,
                        static_char_data_storage,
                        second_column_x,
                    );

                    entity_state(
                        ui,
                        now,
                        inspected_entity_id,
                        char_state_storage,
                        second_column_x,
                    );
                });

            if !opened {
                stop_inspecting_entity_id = Some(*inspected_entity_id);
            }
        }
    }
    if let Some(stop_inspecting_entity_id) = stop_inspecting_entity_id {
        ecs_world
            .write_resource::<ImguiData>()
            .stop_inspecting_entity(stop_inspecting_entity_id);
    }
}

fn entity_state(
    ui: &imgui::Ui,
    now: GameTime<Local>,
    inspected_entity_id: &EntityId<Local>,
    char_state_storage: &mut WriteStorage<LocalCharStateComp<Local>>,
    second_column_x: f32,
) {
    let char_state = char_state_storage
        .get((*inspected_entity_id).into())
        .unwrap();

    ui.text(im_str!("HP"));
    ui.same_line(second_column_x);
    let mut tmp_text = String::with_capacity("99 999".len());
    write_into_str(char_state.hp, &mut tmp_text, 6);
    ui.text(ImString::new(tmp_text));

    ui.text(im_str!("Controllable"));
    ui.same_line(second_column_x);
    if char_state.cannot_control_until.has_already_passed(now) {
        ui.text_colored([0.0, 1.0, 0.0, 1.0], im_str!("True"));
    } else {
        ui.text_colored([1.0, 0.0, 0.0, 1.0], im_str!("False"));
    }

    ui.text(im_str!("CanAttack"));
    ui.same_line(second_column_x);
    if char_state.attack_delay_ends_at.has_already_passed(now) {
        ui.text_colored([0.0, 1.0, 0.0, 1.0], im_str!("True"));
    } else {
        ui.text_colored([1.0, 0.0, 0.0, 1.0], im_str!("False"));
    }

    ui.text(im_str!("Pos"));
    ui.same_line(second_column_x);
    ui.set_next_item_width(150.0);
    let mut pos = [char_state.pos().x, char_state.pos().y];
    ui.input_float2(im_str!("###pos_input"), &mut pos).build();

    ui.text(im_str!("Dir"));
    ui.same_line(second_column_x);
    ui.text(ImString::new(char_state.dir().to_string()));

    let attribs = char_state.calculated_attribs();

    ui.text(im_str!("Max HP"));
    ui.same_line(second_column_x);
    let mut tmp_text = String::with_capacity("99 999".len());
    write_into_str(attribs.max_hp, &mut tmp_text, 6);
    ui.text(ImString::new(tmp_text));

    ui.text(im_str!("HP Regen"));
    ui.same_line(second_column_x);
    ui.text(im_str!("{}%", attribs.hp_regen.as_i16()));

    ui.text(im_str!("Mana Regen"));
    ui.same_line(second_column_x);
    ui.text(im_str!("{}%", attribs.hp_regen.as_i16()));

    ui.text(im_str!("Moving Speed"));
    ui.same_line(second_column_x);
    ui.text(im_str!("{}%", attribs.movement_speed.as_i16()));

    ui.text(im_str!("Armor"));
    ui.same_line(second_column_x);
    ui.text(im_str!("{}%", attribs.armor.as_i16()));

    ui.text(im_str!("Atk"));
    ui.same_line(second_column_x);
    ui.text(im_str!("{}", attribs.attack_damage));

    ui.text(im_str!("Atk Range"));
    ui.same_line(second_column_x);
    ui.text(im_str!("{}%", attribs.attack_range.as_i16()));

    ui.text(im_str!("Atk Speed"));
    ui.same_line(second_column_x);
    ui.text(im_str!("{}%", attribs.attack_speed.as_i16()));

    ui.text(im_str!("Target"));
    ui.bullet();
    ui.text(im_str!(
        "{}",
        char_state
            .target
            .as_ref()
            .map(|it| it.to_string())
            .unwrap_or("None".to_owned())
    ));

    ui.text(im_str!("State"));
    ui.bullet();
    ui.text(im_str!("{}", char_state.state()));
}

fn entity_outlook(
    ui: &imgui::Ui,
    inspected_entity_id: &EntityId<Local>,
    static_char_data_storage: &mut WriteStorage<StaticCharDataComponent>,
    second_column_x: f32,
) {
    let static_data = static_char_data_storage
        .get((*inspected_entity_id).into())
        .unwrap();

    ui.text(im_str!("Job"));
    ui.same_line(second_column_x);
    let mut job_index = JobId::iter()
        .position(|mid| mid == static_data.job_id)
        .unwrap();
    let job_ids: Vec<ImString> = JobId::iter()
        .map(|jid| ImString::new(jid.to_string()))
        .collect();
    ui.set_next_item_width(150.0);
    imgui::ComboBox::new(im_str!("###jobid")).build_simple_string(
        ui,
        &mut job_index,
        job_ids
            .iter()
            .map(|it| it)
            .collect::<Vec<&ImString>>()
            .as_slice(),
    );

    ui.text(im_str!("Outlook"));
    ui.set_next_item_width(150.0);
    ui.same_line(second_column_x);
    match static_data.outlook {
        CharOutlook::Monster(monster_id) => {
            let mut type_index = 0;
            imgui::ComboBox::new(im_str!("###outlook_id")).build_simple_string(
                ui,
                &mut type_index,
                &[im_str!("Monster"), im_str!("Human")],
            );

            ui.text(im_str!("Index"));
            let mut sub_index = MonsterId::iter().position(|mid| mid == monster_id).unwrap();
            let monster_ids: Vec<ImString> = MonsterId::iter()
                .map(|monster_id| ImString::new(monster_id.to_string()))
                .collect();
            ui.set_next_item_width(150.0);
            ui.same_line(second_column_x);
            imgui::ComboBox::new(im_str!("###monster_id")).build_simple_string(
                ui,
                &mut sub_index,
                monster_ids
                    .iter()
                    .map(|it| it)
                    .collect::<Vec<&ImString>>()
                    .as_slice(),
            );
            ui.text(im_str!("")); // empty row
        }
        CharOutlook::Human {
            job_sprite_id,
            head_index,
            sex,
        } => {
            let mut type_index = 1;
            imgui::ComboBox::new(im_str!("###outlook_id")).build_simple_string(
                ui,
                &mut type_index,
                &[im_str!("Monster"), im_str!("Human")],
            );
            let mut job_sprite_index = JobSpriteId::iter()
                .position(|mid| mid == job_sprite_id)
                .unwrap();
            // HEAD COUNT
            // let head_count = world
            //     .read_resource::<SystemVariables>()
            //     .assets
            //     .sprites
            //     .head_sprites[Sex::Male as usize]
            //     .len();
            let human_ids: Vec<ImString> = JobSpriteId::iter()
                .map(|jid| ImString::new(jid.to_string()))
                .collect();
            ui.text(im_str!("JobSpriteId"));
            ui.set_next_item_width(150.0);
            ui.same_line(second_column_x);
            imgui::ComboBox::new(im_str!("###job_spr_id")).build_simple_string(
                ui,
                &mut job_sprite_index,
                human_ids
                    .iter()
                    .map(|it| it)
                    .collect::<Vec<&ImString>>()
                    .as_slice(),
            );

            ui.text(im_str!("Sex"));
            let mut sex_index = 0;
            ui.set_next_item_width(150.0);
            ui.same_line(second_column_x);
            imgui::ComboBox::new(im_str!("###sex_id")).build_simple_string(
                ui,
                &mut sex_index,
                &[im_str!("Male"), im_str!("Female")],
            );
        }
    };
}

fn draw_network_window(data: &mut ImguiData, ui: &imgui::Ui) {
    let mut opened = data.show_network_window;
    Window::new(im_str!("Network"))
        .opened(&mut opened)
        .size([300.0, 700.0], Condition::FirstUseEver)
        .build(&ui, || {
            ui.set_window_font_scale(1.5);
            simple_graph(
                "FPS",
                &data.fps,
                ui,
                0.0,
                140.0,
                0.0,
                [1.0, 0.0, 0.0, 1.0], // red at zero
            );
            simple_graph(
                "Ping",
                &data.pings,
                ui,
                0.0,
                200.0,
                20.0,
                [0.0, 1.0, 0.0, 1.0], // green at zero
            );
            simple_graph(
                "in Packets/s",
                &data.inc_packets_per_second,
                ui,
                0.0,
                200.0,
                0.0,
                [1.0, 0.0, 0.0, 1.0],
            );
            simple_graph(
                "in bytes/s",
                &data.incoming_bytes_per_second,
                ui,
                0.0,
                200.0,
                0.0,
                [1.0, 0.0, 0.0, 1.0],
            );
            simple_graph(
                "out Packets/s",
                &data.out_packets_per_second,
                ui,
                0.0,
                200.0,
                0.0,
                [1.0, 0.0, 0.0, 1.0],
            );
            simple_graph(
                "out bytes/s",
                &data.outgoing_bytes_per_second,
                ui,
                0.0,
                200.0,
                0.0,
                [1.0, 0.0, 0.0, 1.0],
            );
            simple_value(
                "UnAcked Predictions",
                data.unacked_prediction_count,
                ui,
                0.0,
                20.0,
                1.0,
                [1.0, 0.0, 0.0, 1.0], // red at zero
            );
            rollback_graph(&data.rollbacks_per_second, ui);
            simple_value(
                "Simulations/s",
                data.simulation_duration,
                ui,
                0.0,
                (SIMULATION_FREQ + 20) as f32,
                0.0,
                [1.0, 0.0, 0.0, 1.0],
            );
        });
    data.show_network_window = opened;
}

fn rollback_graph(data: &[f32], ui: &imgui::Ui) {
    let cur_value = *data.last().unwrap();
    let caption = im_str!("Rollbacks {}", cur_value);
    let max = SIMULATION_FREQ as f32;

    let color = color_gradient(
        0.0,   // start
        max,   // end
        -10.0, // start_offset
        cur_value,
        [0.0, 1.0, 0.0, 1.0], // start color
        [1.0, 0.0, 0.0, 1.0], // end color
    );
    let color_token = ui.push_style_color(StyleColor::PlotHistogram, color.clone());
    let color2 = ui.push_style_color(StyleColor::Text, color);
    ui.plot_histogram(im_str!(""), data)
        .overlay_text(&caption)
        .graph_size([(ui.window_size()[0]) * 0.9, 50.0])
        .scale_min(0.0)
        .scale_max(max)
        .build();

    color_token.pop(ui);
    color2.pop(ui);
}

fn simple_value(
    caption: &str,
    cur_value: f32,
    ui: &imgui::Ui,
    min_value: f32,
    max_value: f32,
    offset: f32,
    at_zero_color: [f32; 4],
) {
    let color = if cur_value <= min_value {
        at_zero_color
    } else {
        color_gradient(
            min_value, // start
            max_value, // end
            offset,    // start_offset
            cur_value,
            [0.0, 1.0, 0.0, 1.0], // start color
            [1.0, 0.0, 0.0, 1.0], // end color
        )
    };
    let caption = im_str!("{}: {:.0}", caption, cur_value);
    ui.text_colored(color, caption);
}

fn simple_graph(
    caption: &str,
    data: &[f32],
    ui: &imgui::Ui,
    min_value: f32,
    max_value: f32,
    offset: f32,
    at_zero_color: [f32; 4],
) {
    let cur_value = *data.last().unwrap();
    let caption = im_str!("{} {}", caption, cur_value);

    let color = if cur_value <= min_value {
        at_zero_color
    } else {
        color_gradient(
            min_value, // start
            max_value, // end
            offset,    // start_offset
            cur_value,
            [0.0, 1.0, 0.0, 1.0], // start color
            [1.0, 0.0, 0.0, 1.0], // end color
        )
    };
    let color_token = ui.push_style_color(StyleColor::PlotHistogram, color.clone());
    let color2 = ui.push_style_color(StyleColor::Text, color);
    ui.plot_lines(im_str!(""), data)
        .overlay_text(&caption)
        .graph_size([(ui.window_size()[0]) * 0.9, 50.0])
        .scale_min(min_value)
        .scale_max(max_value)
        .build();

    color_token.pop(ui);
    color2.pop(ui);
}

fn color_gradient(
    min: f32,
    max: f32,
    start_offset: f32,
    value: f32,
    start_color: [f32; 4],
    end_color: [f32; 4],
) -> [f32; 4] {
    let mid_color = [
        (start_color[0] + end_color[0]) / 2_f32,
        (start_color[1] + end_color[1]) / 2_f32,
        (start_color[2] + end_color[2]) / 2_f32,
        (start_color[3] + end_color[3]) / 2_f32,
    ];
    let range = (max - min).abs();
    let normalized = 1.0 - (value - start_offset).max(min).min(max) / range;
    return if normalized > 0.5 {
        // yellow to green
        let perc = (normalized - 0.5) * 2.0;
        color_grad(mid_color, start_color, perc)
    } else {
        // red to yellow
        let perc = normalized * 2.0;
        color_grad(end_color, mid_color, perc)
    };
}

fn color_grad(from_color: [f32; 4], to_color: [f32; 4], perc: f32) -> [f32; 4] {
    let from_r = 1.0;
    let from_g = 1.0;
    let to_r = 0.0;
    let to_g = 1.0;
    let r_dir = to_color[0] - from_color[0];
    let g_dir = to_color[1] - from_color[1];
    let b_dir = to_color[2] - from_color[2];
    let a_dir = to_color[3] - from_color[3];

    [
        from_color[0] + r_dir * perc,
        from_color[1] + g_dir * perc,
        from_color[2] + b_dir * perc,
        from_color[3] + a_dir * perc,
    ]
}
