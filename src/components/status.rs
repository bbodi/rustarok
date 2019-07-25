use specs::prelude::*;
use crate::ElapsedTime;
use crate::components::char::{CharAttributes, Percentage, CharacterStateComponent, CharOutlook, U8Float};
use std::sync::{Arc, Mutex};
use specs::{Entity, LazyUpdate};
use crate::consts::JobId;
use crate::systems::{Sex, Sprites};
use crate::asset::SpriteResource;

pub trait Status {
    fn can_target_move(&self) -> bool;
    fn can_target_cast(&self) -> bool;
    //    fn get_render_effect(&self) ->;
    fn get_render_color(&self) -> [f32; 4];
    fn get_render_size(&self) -> f32;
    fn calc_attribs(&self, attributes: &mut CharAttributes);
    fn calc_render_sprite<'a>(
        &self,
        job_id: JobId,
        head_index: usize,
        sex: Sex,
        sprites: &'a Sprites,
    ) -> Option<&'a SpriteResource>;
    fn update(
        &mut self,
        now: ElapsedTime,
        char_state: &mut CharacterStateComponent,
    ) -> StatusUpdateResult;
    fn get_duration_percent_for_rendering(&self) -> Option<f32>;
}

#[derive(Debug)]
pub enum MainStatus {
    Mounted,
    Stun,
}

pub struct Statuses {
    statuses: [Option<Arc<Mutex<Box<dyn Status>>>>; 32],
}

unsafe impl Sync for Statuses {}

unsafe impl Send for Statuses {}

impl Statuses {
    pub fn new() -> Statuses {
        Statuses {
            statuses: Default::default()
        }
    }

    pub fn update(
        &mut self,
        now: ElapsedTime,
        char_state: &mut CharacterStateComponent,
    ) {
        for status in self.statuses.iter_mut().filter(|it| it.is_some()) {
            status.as_ref().unwrap().lock().unwrap().update(now, char_state);
        }
    }

    pub fn get_base_attributes(outlook: &CharOutlook) -> CharAttributes {
        return match outlook {
            CharOutlook::Player { job_id, head_index, sex } => {
                match job_id {
                    _ => CharAttributes {
                        walking_speed: U8Float::new(Percentage::new(100.0)),
                        attack_range: U8Float::new(Percentage::new(100.0)),
                        attack_speed: U8Float::new(Percentage::new(100.0)),
                        attack_damage: 76,
                        armor: U8Float::new(Percentage::new(10.0)),
                        max_hp: 2000,
                    }
                }
            }
            CharOutlook::Monster(monster_id) => {
                match monster_id {
                    _ => CharAttributes {
                        walking_speed: U8Float::new(Percentage::new(100.0)),
                        attack_range: U8Float::new(Percentage::new(100.0)),
                        attack_speed: U8Float::new(Percentage::new(100.0)),
                        attack_damage: 76,
                        armor: U8Float::new(Percentage::new(0.0)),
                        max_hp: 2000,
                    }
                }
            }
        };
    }

    pub fn calc_attribs(&self, outlook: &CharOutlook) -> CharAttributes {
        let mut calculated_attribs = Statuses::get_base_attributes(outlook);
        for status in &mut self.statuses.iter().filter(|it| it.is_some()) {
            status.as_ref().unwrap().lock().unwrap().calc_attribs(&mut calculated_attribs);
        }
        return calculated_attribs;
    }

    pub fn calc_render_sprite<'a>(
        &self,
        job_id: JobId,
        head_index: usize,
        sex: Sex,
        sprites: &'a Sprites,
    ) -> &'a SpriteResource {
        let mut sprite = {
            let sprites = &sprites.character_sprites;
            &sprites[&job_id][sex as usize]
        };
        for status in &mut self.statuses.iter().filter(|it| it.is_some()) {
            if let Some(spr) = status.as_ref().unwrap().lock().unwrap().calc_render_sprite(
                job_id,
                head_index,
                sex,
                sprites,
            ) {
                sprite = spr;
            }
        }
        return sprite;
    }

    pub fn is_mounted(&self) -> bool {
        self.statuses[MainStatus::Mounted as usize].is_some()
    }

    pub fn is_stunned(&self) -> bool {
        self.statuses[MainStatus::Stun as usize].is_some()
    }

    pub fn switch_mounted(&mut self) {
        let is_mounted = self.statuses[MainStatus::Mounted as usize].is_some();
        let value: Option<Arc<Mutex<Box<dyn Status>>>> = if !is_mounted {
            Some(Arc::new(Mutex::new(Box::new(MountedStatus {}))))
        } else {
            None
        };
        self.statuses[MainStatus::Mounted as usize] = value;
    }
}

struct MountedStatus;

pub enum StatusUpdateResult {
    RemoveIt,
    KeepIt,
}

impl Status for MountedStatus {
    fn can_target_move(&self) -> bool { true }

    fn can_target_cast(&self) -> bool { true }

    fn get_render_color(&self) -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }

    fn get_render_size(&self) -> f32 {
        1.0
    }

    fn calc_attribs(&self, attributes: &mut CharAttributes) {
        // it is applied directly on the base moving speed, since it is called first
        attributes.walking_speed.increase_by(Percentage::new(200.0));
    }

    fn update(&mut self,
              now: ElapsedTime,
              char_state: &mut CharacterStateComponent,
    ) -> StatusUpdateResult {
        StatusUpdateResult::KeepIt
    }

    fn get_duration_percent_for_rendering(&self) -> Option<f32> {
        None
    }

    fn calc_render_sprite<'a>(
        &self,
        job_id: JobId,
        head_index: usize,
        sex: Sex,
        sprites: &'a Sprites,
    ) -> Option<&'a SpriteResource> {
        Some(&sprites.mounted_character_sprites[&job_id][sex as usize])
    }
}

enum ApplyStatusComponentPayload {
    MainStatus(MainStatus),
    SecondaryStatus(Arc<Mutex<Box<dyn Status>>>),
}

#[derive(Component)]
pub struct ApplyStatusComponent {
    target_entity_id: Entity,
    status: ApplyStatusComponentPayload,
}

unsafe impl Sync for ApplyStatusComponent {}

unsafe impl Send for ApplyStatusComponent {}

impl ApplyStatusComponent {
    pub fn from_main_status(target_entity_id: Entity, m: MainStatus) -> ApplyStatusComponent {
        ApplyStatusComponent {
            target_entity_id,
            status: ApplyStatusComponentPayload::MainStatus(m),
        }
    }

    pub fn from_secondary_status(target_entity_id: Entity, status: Box<dyn Status>) -> ApplyStatusComponent {
        ApplyStatusComponent {
            target_entity_id,
            status: ApplyStatusComponentPayload::SecondaryStatus(Arc::new(Mutex::new(status))),
        }
    }
}

pub struct ApplyStatusSystem;

impl<'a> specs::System<'a> for ApplyStatusSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, ApplyStatusComponent>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(&mut self, (
        entities,
        mut char_state_storage,
        mut statuses,
        mut updater,
    ): Self::SystemData) {
        for (status_entity_id, status) in (&entities,
                                           &mut statuses).join() {
            let status: &ApplyStatusComponent = status;
            if let Some(target_char) = char_state_storage.get_mut(status.target_entity_id) {
                match &status.status {
                    ApplyStatusComponentPayload::MainStatus(status_name) => {
                        log::debug!("Applying state '{:?}' on {:?}", status_name, status.target_entity_id);
                        match status_name {
                            _ => {
                                target_char.statuses.switch_mounted();
                            }
                        }
                    }
                    ApplyStatusComponentPayload::SecondaryStatus(box_status) => {}
                }
                target_char.calculated_attribs = target_char
                    .statuses
                    .calc_attribs(&target_char.outlook);
            }
            updater.remove::<ApplyStatusComponent>(status_entity_id);
        }
    }
}