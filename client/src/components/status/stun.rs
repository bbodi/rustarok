use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::status::status::{StatusUpdateParams, StatusUpdateResult};
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::render_action;
use crate::systems::AssetResources;
use crate::GameTime;
use rustarok_common::common::{Local, Vec2};
use rustarok_common::components::char::{CharDir, EntityId};
use specs::{Entities, LazyUpdate};

#[derive(Clone, Debug)]
pub struct StunStatus {
    pub caster_entity_id: EntityId<Local>,
    pub started: GameTime<Local>,
    pub until: GameTime<Local>,
}

impl StunStatus {
    pub fn new(
        caster_entity_id: EntityId<Local>,
        now: GameTime<Local>,
        duration: f32,
    ) -> StunStatus {
        StunStatus {
            caster_entity_id,
            started: now,
            until: now.add_seconds(duration),
        }
    }
}

impl StunStatus {
    pub fn on_apply(
        &mut self,
        self_entity_id: EntityId<Local>,
        target_char: &mut CharacterStateComponent,
        entities: &Entities,
        updater: &mut LazyUpdate,
        assets: &AssetResources,
        now: GameTime<Local>,
    ) {
        // TODO2
        //        target_char.set_state(ClientCharState::StandBy, target_char.dir());
        //        let entity = entities.create();
        //        updater.insert(
        //            entity,
        //            SoundEffectComponent {
        //                target_entity_id: self_entity_id,
        //                sound_id: assets.sounds.stun,
        //                pos: target_char.pos(),
        //                start_time: now,
        //            },
        //        );
    }

    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if self.until.has_already_passed(params.time.now()) {
            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }

    pub fn render(
        &self,
        char_pos: Vec2,
        now: GameTime<Local>,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
    ) {
        let anim = SpriteRenderDescriptorComponent {
            action_index: CharActionIndex::Idle as usize,
            animation_started: self.started,
            animation_ends_at: GameTime::from(0.0),
            forced_duration: None,
            direction: CharDir::South,
            fps_multiplier: 1.0,
        };
        render_action(
            now,
            &anim,
            &assets.sprites.stun,
            &char_pos,
            [0, -100],
            false,
            1.0,
            ActionPlayMode::Repeat,
            &[255, 255, 255, 255],
            render_commands,
        );
    }

    pub fn get_status_completion_percent(
        &self,
        now: GameTime<Local>,
    ) -> Option<(GameTime<Local>, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }
}
