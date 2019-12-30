use crate::systems::console_commands::humanize_bytes;
use crate::SIMULATION_FREQ;
use imgui::*;

pub struct ImguiSys;

const PING_COUNT: usize = 16;
pub struct ImguiData {
    pings: [f32; PING_COUNT],
    unacked_prediction_count: f32,
    rollbacks_per_second: [f32; PING_COUNT],
    fps: [f32; PING_COUNT],
    inc_packets_per_second: [f32; PING_COUNT],
    out_packets_per_second: [f32; PING_COUNT],
    incoming_bytes_per_second: [f32; PING_COUNT],
    outgoing_bytes_per_second: [f32; PING_COUNT],
    simulation_duration: f32,
}

impl ImguiData {
    pub fn new() -> ImguiData {
        ImguiData {
            pings: [0.0; PING_COUNT],
            fps: [0.0; PING_COUNT],
            inc_packets_per_second: [0.0; PING_COUNT],
            out_packets_per_second: [0.0; PING_COUNT],
            incoming_bytes_per_second: [0.0; PING_COUNT],
            outgoing_bytes_per_second: [0.0; PING_COUNT],
            rollbacks_per_second: [0.0; PING_COUNT],
            unacked_prediction_count: 0.0,
            simulation_duration: 0.0,
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
}

pub fn draw_imgui(data: &mut ImguiData, ui: &imgui::Ui) {
    ui.show_demo_window(&mut true);

    Window::new(im_str!("Network"))
        .size([300.0, 500.0], Condition::FirstUseEver)
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
    let normalized = 1.0 - (value - start_offset).max(min).min(max) / max;
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
