use crate::asset::SpriteResource;
use crate::components::char::{
    percentage, ActionPlayMode, CharAttributeModifier, CharAttributeModifierCollector,
    CharAttributes, CharacterStateComponent, Percentage, Team,
};
use crate::components::controller::CharEntityId;
use crate::components::{
    ApplyForceComponent, HpModificationRequest, HpModificationResult, HpModificationType,
};
use crate::configs::DevConfig;
use crate::consts::JobId;
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::RenderDesktopClientSystem;
use crate::systems::{Sex, SystemVariables};
use crate::ElapsedTime;
use nalgebra::Isometry2;
use specs::{Entities, LazyUpdate};
use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::ops::Deref;
use strum_macros::EnumCount;

#[derive(Debug)]
pub enum StatusStackingResult {
    DontAddTheNewStatus,
    AddTheNewStatus,
    Replace,
}

pub trait Status: Any {
    fn dupl(&self) -> Box<dyn Status + Send>;

    fn get_body_sprite<'a>(
        &self,
        sys_vars: &'a SystemVariables,
        job_id: JobId,
        sex: Sex,
    ) -> Option<&'a SpriteResource> {
        None
    }

    fn on_apply(
        &mut self,
        self_entity_id: CharEntityId,
        target_char: &mut CharacterStateComponent,
        entities: &Entities,
        updater: &mut LazyUpdate,
        sys_vars: &SystemVariables,
        physics_world: &mut PhysicEngine,
    ) {
    }

    fn can_target_move(&self) -> bool {
        true
    }

    fn can_target_be_controlled(&self) -> bool {
        true
    }

    fn can_target_cast(&self) -> bool {
        true
    }

    fn get_render_color(&self, _now: ElapsedTime) -> [u8; 4] {
        [255, 255, 255, 255]
    }

    fn get_render_size(&self) -> f32 {
        1.0
    }

    fn calc_attribs(&self, _modifiers: &mut CharAttributeModifierCollector) {}

    fn update(
        &mut self,
        _self_char_id: CharEntityId,
        _target_char: &mut CharacterStateComponent,
        _phyisic_world: &mut PhysicEngine,
        _system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut LazyUpdate,
    ) -> StatusUpdateResult {
        StatusUpdateResult::KeepIt
    }

    fn hp_mod_is_calculated_but_not_applied_yet(
        &mut self,
        outcome: HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) -> HpModificationResult {
        outcome
    }

    fn hp_mod_has_been_applied_on_me(
        &mut self,
        self_id: CharEntityId,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
    }

    fn hp_mod_has_been_applied_on_enemy(
        &mut self,
        self_id: CharEntityId,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
    }

    fn allow_push(&self, _push: &ApplyForceComponent) -> bool {
        true
    }

    fn render(
        &self,
        _char_state: &CharacterStateComponent,
        _system_vars: &SystemVariables,
        _render_commands: &mut RenderCommandCollector,
    ) {
    }

    fn get_status_completion_percent(&self, _now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        None
    }

    fn stack(&self, _other: &Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::AddTheNewStatus
    }

    fn typ(&self) -> StatusNature;
}

// TODO: should 'Dead' be a status?
#[derive(Debug, EnumCount, Clone, Copy)]
pub enum MainStatuses {
    Mounted,
}

#[derive(Debug, Clone, Copy)]
pub enum MainStatusesIndex {
    Mounted,
}

#[derive(Clone)]
pub struct MountedStatus {
    speedup: Percentage,
}

const STATUS_ARRAY_SIZE: usize = 32;
pub struct Statuses {
    statuses: [Option<Box<dyn Status>>; STATUS_ARRAY_SIZE],
    first_free_index: usize,
    cached_modifier_collector: CharAttributeModifierCollector,
}

unsafe impl Sync for Statuses {}

unsafe impl Send for Statuses {}

impl Statuses {
    pub fn new() -> Statuses {
        Statuses {
            statuses: Default::default(),
            first_free_index: MAINSTATUSES_COUNT,
            cached_modifier_collector: CharAttributeModifierCollector::new(),
        }
    }

    pub fn can_move(&self) -> bool {
        let mut allow = true;
        for status in self
            .statuses
            .iter()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            allow &= status.as_ref().unwrap().can_target_move();
        }
        return allow;
    }

    pub fn can_cast(&self) -> bool {
        let mut allow = true;
        for status in self
            .statuses
            .iter()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            allow &= status.as_ref().unwrap().can_target_cast();
        }
        return allow;
    }

    pub fn can_be_controlled(&self) -> bool {
        let mut allow = true;
        for status in self
            .statuses
            .iter()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            allow &= status.as_ref().unwrap().can_target_be_controlled();
        }
        return allow;
    }

    pub fn allow_push(&mut self, push: &ApplyForceComponent) -> bool {
        let mut allow = true;
        for status in self
            .statuses
            .iter_mut()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            allow &= status.as_ref().unwrap().allow_push(push);
        }
        return allow;
    }

    pub fn hp_mod_has_been_applied_on_enemy(
        &mut self,
        self_id: CharEntityId,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
        for status in self
            .statuses
            .iter_mut()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            status.as_mut().unwrap().hp_mod_has_been_applied_on_enemy(
                self_id,
                &outcome,
                hp_mod_reqs,
            );
        }
    }

    pub fn hp_mod_is_calculated_but_not_applied_yet(
        &mut self,
        mut outcome: HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) -> HpModificationResult {
        for status in self
            .statuses
            .iter_mut()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            outcome = status
                .as_mut()
                .unwrap()
                .hp_mod_is_calculated_but_not_applied_yet(outcome, hp_mod_reqs);
        }
        return outcome;
    }

    pub fn hp_mod_has_been_applied_on_me(
        &mut self,
        self_id: CharEntityId,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
        for status in self
            .statuses
            .iter_mut()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            status.as_mut().unwrap().hp_mod_has_been_applied_on_enemy(
                self_id,
                &outcome,
                hp_mod_reqs,
            );
        }
    }

    pub fn update(
        &mut self,
        self_char_id: CharEntityId,
        char_state: &mut CharacterStateComponent,
        physics_world: &mut PhysicEngine,
        sys_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut LazyUpdate,
    ) -> u32 {
        let mut changed: u32 = 0;
        for (i, status) in self
            .statuses
            .iter_mut()
            .enumerate()
            .take(self.first_free_index)
            .filter(|(i, it)| it.is_some())
        {
            let result = status.as_mut().unwrap().update(
                self_char_id,
                char_state,
                physics_world,
                sys_vars,
                entities,
                updater,
            );
            match result {
                StatusUpdateResult::RemoveIt => {
                    changed |= 1 << i;
                }
                StatusUpdateResult::KeepIt => {}
            }
        }

        return changed;
    }

    pub fn remove_statuses(&mut self, bit_indices: u32) {
        for i in 0..32 {
            if ((1 << i) & bit_indices) > 0 {
                self.statuses[i] = None;
            }
        }
        while self.first_free_index > MAINSTATUSES_COUNT
            && self.statuses[self.first_free_index - 1].is_none()
        {
            self.first_free_index -= 1;
        }
    }

    pub fn render(
        &self,
        char_pos: &CharacterStateComponent,
        sys_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
        let mut already_rendered = HashSet::with_capacity(self.statuses.len());
        for status in self.statuses.iter().filter(|it| it.is_some()) {
            let boxx = status.as_ref().unwrap();
            let type_id = boxx.deref().type_id();
            if !already_rendered.contains(&type_id) {
                boxx.render(char_pos, sys_vars, render_commands);
                already_rendered.insert(type_id);
            }
        }
    }

    pub fn get_base_attributes(job_id: JobId, configs: &DevConfig) -> CharAttributes {
        return match job_id {
            JobId::CRUSADER => configs.stats.player.crusader.attributes.clone(),
            JobId::GUNSLINGER => configs.stats.player.gunslinger.attributes.clone(),
            JobId::RANGER => configs.stats.player.hunter.attributes.clone(),
            JobId::RangedMinion => configs.stats.minion.ranged.clone(),
            JobId::HealingDummy => CharAttributes {
                movement_speed: percentage(0),
                attack_range: percentage(0),
                attack_speed: percentage(0),
                attack_damage: 0,
                armor: percentage(0),
                healing: percentage(100),
                hp_regen: percentage(0),
                max_hp: 1_000_000,
                mana_regen: percentage(0),
            },
            JobId::TargetDummy => CharAttributes {
                movement_speed: percentage(0),
                attack_range: percentage(0),
                attack_speed: percentage(0),
                attack_damage: 0,
                armor: percentage(0),
                healing: percentage(100),
                hp_regen: percentage(0),
                max_hp: 1_000_000,
                mana_regen: percentage(0),
            },
            JobId::MeleeMinion => configs.stats.minion.melee.clone(),
            JobId::Turret => configs.skills.gaz_turret.turret.clone(),
            JobId::Barricade => {
                let configs = &configs.skills.gaz_barricade;
                CharAttributes {
                    movement_speed: percentage(0),
                    attack_range: percentage(0),
                    attack_speed: percentage(0),
                    attack_damage: 0,
                    armor: configs.armor,
                    healing: percentage(0),
                    hp_regen: configs.hp_regen,
                    max_hp: configs.max_hp,
                    mana_regen: percentage(10),
                }
            }
            _ => CharAttributes {
                movement_speed: percentage(100),
                attack_range: percentage(100),
                attack_speed: percentage(100),
                attack_damage: 76,
                armor: percentage(10),
                healing: percentage(100),
                hp_regen: percentage(100),
                max_hp: 2000,
                mana_regen: percentage(100),
            },
        };
    }

    pub fn calc_attributes(&mut self) -> &CharAttributeModifierCollector {
        self.cached_modifier_collector.clear();
        for status in &mut self
            .statuses
            .iter()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            status
                .as_ref()
                .unwrap()
                .calc_attribs(&mut self.cached_modifier_collector);
        }
        return &self.cached_modifier_collector;
    }

    pub fn calc_body_sprite<'a>(
        &self,
        sys_vars: &'a SystemVariables,
        job_id: JobId,
        sex: Sex,
    ) -> Option<&'a SpriteResource> {
        let mut ret = None;
        for status in &mut self
            .statuses
            .iter()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            let body = status
                .as_ref()
                .unwrap()
                .get_body_sprite(sys_vars, job_id, sex);
            if body.is_some() {
                ret = body;
            }
        }
        return ret;
    }

    pub fn calc_render_color(&self, now: ElapsedTime) -> [u8; 4] {
        let mut ret = [255, 255, 255, 255];
        for status in &mut self
            .statuses
            .iter()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            let status_color = status.as_ref().unwrap().get_render_color(now);
            for i in 0..4 {
                ret[i] = (ret[i] as u32 * status_color[i] as u32 / 255) as u8;
            }
        }
        return ret;
    }

    pub fn calc_largest_remaining_status_time_percent(&self, now: ElapsedTime) -> Option<f32> {
        let mut ret: Option<(ElapsedTime, f32)> = None;
        for status in &mut self
            .statuses
            .iter()
            .take(self.first_free_index)
            .filter(|it| it.is_some())
        {
            let rem: Option<(ElapsedTime, f32)> =
                status.as_ref().unwrap().get_status_completion_percent(now);
            ret = if let Some((status_ends_at, _status_remaining_time)) = rem {
                if let Some((current_ends_at, _current_rem_time)) = ret {
                    if current_ends_at.has_not_passed_yet(status_ends_at) {
                        rem
                    } else {
                        ret
                    }
                } else {
                    rem
                }
            } else {
                ret
            };
        }
        return ret.map(|it| it.1);
    }

    pub fn is_mounted(&self) -> bool {
        self.statuses[MainStatusesIndex::Mounted as usize].is_some()
    }

    pub fn switch_mounted(&mut self, mounted_speedup: Percentage) {
        let is_mounted = self.statuses[MainStatusesIndex::Mounted as usize].is_some();
        let value: Option<Box<dyn Status>> = if !is_mounted {
            Some(Box::new(MountedStatus {
                speedup: mounted_speedup,
            }))
        } else {
            None
        };
        self.statuses[MainStatusesIndex::Mounted as usize] = value;
    }

    pub fn add(&mut self, new_status: Box<dyn Status>) {
        if self.first_free_index >= STATUS_ARRAY_SIZE {
            log::error!("There is no more space for new Status!");
            return;
        }
        let type_id = new_status.as_ref().type_id();
        let (current_index, stack_type) = self
            .statuses
            .iter()
            .take(self.first_free_index)
            .enumerate()
            .find(|(i, current_status)| {
                current_status
                    .as_ref()
                    .map(|current_status| type_id == current_status.as_ref().type_id())
                    .unwrap_or(false)
            })
            .map(|(i, current_status)| (i, current_status.as_ref().unwrap().stack(&new_status)))
            .unwrap_or((0, StatusStackingResult::AddTheNewStatus));
        match stack_type {
            StatusStackingResult::Replace => {
                self.statuses[current_index] = Some(new_status);
            }
            StatusStackingResult::AddTheNewStatus => {
                self.statuses[self.first_free_index] = Some(new_status);
                self.first_free_index += 1;
            }
            StatusStackingResult::DontAddTheNewStatus => {
                return;
            }
        }
    }

    pub fn remove_all(&mut self) {
        for status in self.statuses.iter_mut().take(self.first_free_index) {
            *status = None;
        }
        self.first_free_index = MAINSTATUSES_COUNT;
    }

    pub fn remove<T: 'static, P>(&mut self, predicate: P)
    where
        P: Fn(&T) -> bool,
    {
        let removing_type_id = std::any::TypeId::of::<T>();
        for arc_status in self.statuses.iter_mut().take(self.first_free_index) {
            let should_remove = arc_status
                .as_ref()
                .map(|boxx| {
                    let type_id = boxx.as_ref().type_id();
                    type_id == removing_type_id
                        && unsafe { predicate(Statuses::trait_to_struct(boxx)) }
                })
                .unwrap_or(false);
            if should_remove {
                *arc_status = None;
            }
        }
    }

    pub fn remove_by_nature(&mut self, status_type: StatusNature) {
        for arc_status in self.statuses.iter_mut().take(self.first_free_index) {
            let should_remove = arc_status
                .as_ref()
                .map(|it| it.typ() == status_type)
                .unwrap_or(false);
            if should_remove {
                *arc_status = None;
            }
        }
    }

    pub fn remove_main_status(&mut self, status: MainStatusesIndex) {
        self.statuses[status as usize] = None;
    }

    unsafe fn trait_to_struct<T>(boxx: &Box<dyn Status>) -> &T {
        return std::mem::transmute::<_, &Box<T>>(boxx);
    }

    pub fn get_status<T: 'static>(&self) -> Option<&T> {
        let requested_type_id = TypeId::of::<T>();
        for status in self.statuses.iter().filter(|it| it.is_some()) {
            let boxx: &Box<dyn Status> = &status.as_ref().unwrap();
            let type_id = boxx.as_ref().type_id();
            if requested_type_id == type_id {
                let param: &T = unsafe { Statuses::trait_to_struct(boxx) };
                return Some(param);
            }
        }
        return None;
    }

    pub fn with_status<F, T: 'static, R>(&self, func: F) -> Option<R>
    where
        F: Fn(&T) -> R,
    {
        let requested_type_id = TypeId::of::<T>();
        for status in self.statuses.iter().filter(|it| it.is_some()) {
            let boxx: &Box<dyn Status> = &status.as_ref().unwrap();
            let type_id = boxx.as_ref().type_id();
            if requested_type_id == type_id {
                let param: &T = unsafe { Statuses::trait_to_struct(boxx) };
                return Some(func(param));
            }
        }
        return None;
    }

    pub fn count(&self) -> usize {
        let secondary_status_count = self.first_free_index - MAINSTATUSES_COUNT;
        let main_status_count = self
            .statuses
            .iter()
            .take(MAINSTATUSES_COUNT)
            .filter(|it| it.is_some())
            .count();
        return main_status_count + secondary_status_count;
    }
}

pub enum StatusUpdateResult {
    RemoveIt,
    KeepIt,
}

impl Status for MountedStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn get_body_sprite<'a>(
        &self,
        sys_vars: &'a SystemVariables,
        job_id: JobId,
        sex: Sex,
    ) -> Option<&'a SpriteResource> {
        let sprites = &sys_vars.assets.sprites;
        sprites
            .mounted_character_sprites
            .get(&job_id)
            .and_then(|it| it.get(sex as usize))
    }

    fn calc_attribs(&self, modifiers: &mut CharAttributeModifierCollector) {
        // it is applied directly on the base moving speed, since it is called first
        modifiers.change_walking_speed(
            CharAttributeModifier::IncreaseByPercentage(self.speedup),
            ElapsedTime(0.0),
            ElapsedTime(0.0),
        );
    }

    fn stack(&self, _other: &Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::DontAddTheNewStatus
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Supportive
    }
}

#[derive(Clone)]
pub struct PoisonStatus {
    pub poison_caster_entity_id: CharEntityId,
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub next_damage_at: ElapsedTime,
    pub damage: u32,
}

impl Status for PoisonStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn get_render_color(&self, _now: ElapsedTime) -> [u8; 4] {
        [128, 255, 128, 255]
    }

    fn update(
        &mut self,
        self_char_id: CharEntityId,
        _char_state: &mut CharacterStateComponent,
        _physics_world: &mut PhysicEngine,
        sys_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _updater: &mut LazyUpdate,
    ) -> StatusUpdateResult {
        if self.until.has_already_passed(sys_vars.time) {
            StatusUpdateResult::RemoveIt
        } else {
            if self.next_damage_at.has_already_passed(sys_vars.time) {
                sys_vars.hp_mod_requests.push(HpModificationRequest {
                    src_entity: self.poison_caster_entity_id,
                    dst_entity: self_char_id,
                    typ: HpModificationType::Poison(30),
                });
                self.next_damage_at = sys_vars.time.add_seconds(1.0);
            }
            StatusUpdateResult::KeepIt
        }
    }

    fn render(
        &self,
        char_state: &CharacterStateComponent,
        sys_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Quagmire,
            self.started,
            &char_state.pos(),
            sys_vars,
            render_commands,
            ActionPlayMode::Repeat,
        );
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }

    fn stack(&self, _other: &Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::Replace
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Harmful
    }
}

pub enum ApplyStatusComponentPayload {
    MainStatus(MainStatuses),
    SecondaryStatus(Box<dyn Status + Send>),
}

impl ApplyStatusComponentPayload {
    pub fn from_main_status(m: MainStatuses) -> ApplyStatusComponentPayload {
        ApplyStatusComponentPayload::MainStatus(m)
    }

    pub fn from_secondary(status: Box<dyn Status + Send>) -> ApplyStatusComponentPayload {
        ApplyStatusComponentPayload::SecondaryStatus(status)
    }
}

impl Clone for ApplyStatusComponentPayload {
    fn clone(&self) -> Self {
        match self {
            ApplyStatusComponentPayload::MainStatus(m) => {
                ApplyStatusComponentPayload::MainStatus(*m)
            }
            ApplyStatusComponentPayload::SecondaryStatus(arc) => {
                let boxed_status_clone = arc.dupl();
                ApplyStatusComponentPayload::SecondaryStatus(boxed_status_clone)
            }
        }
    }
}

pub struct ApplyStatusComponent {
    pub source_entity_id: CharEntityId,
    pub target_entity_id: CharEntityId,
    pub status: ApplyStatusComponentPayload,
}

pub struct ApplyStatusInAreaComponent {
    pub source_entity_id: CharEntityId,
    pub status: ApplyStatusComponentPayload,
    pub area_shape: Box<dyn ncollide2d::shape::Shape<f32>>,
    pub area_isom: Isometry2<f32>,
    pub except: Option<CharEntityId>,
    pub nature: StatusNature,
    pub caster_team: Team,
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum StatusNature {
    Supportive,
    Harmful,
    Neutral,
}

pub enum RemoveStatusComponentPayload {
    MainStatus(MainStatusesIndex),
    RemovingStatusType(StatusNature),
}

pub struct RemoveStatusComponent {
    pub source_entity_id: CharEntityId,
    pub target_entity_id: CharEntityId,
    pub status: RemoveStatusComponentPayload,
}

unsafe impl Sync for ApplyStatusComponent {}

unsafe impl Send for ApplyStatusComponent {}

unsafe impl Sync for ApplyStatusInAreaComponent {}

unsafe impl Send for ApplyStatusInAreaComponent {}

impl ApplyStatusComponent {
    pub fn from_main_status(
        source_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        m: MainStatuses,
    ) -> ApplyStatusComponent {
        ApplyStatusComponent {
            source_entity_id,
            target_entity_id,
            status: ApplyStatusComponentPayload::MainStatus(m),
        }
    }

    pub fn from_secondary_status(
        source_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        status: Box<dyn Status + Send>,
    ) -> ApplyStatusComponent {
        ApplyStatusComponent {
            source_entity_id,
            target_entity_id,
            status: ApplyStatusComponentPayload::from_secondary(status),
        }
    }
}

impl RemoveStatusComponent {
    pub fn from_main_status(
        source_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        m: MainStatusesIndex,
    ) -> RemoveStatusComponent {
        RemoveStatusComponent {
            source_entity_id,
            target_entity_id,
            status: RemoveStatusComponentPayload::MainStatus(m),
        }
    }

    pub fn by_status_nature(
        source_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        status_type: StatusNature,
    ) -> RemoveStatusComponent {
        RemoveStatusComponent {
            source_entity_id,
            target_entity_id,
            status: RemoveStatusComponentPayload::RemovingStatusType(status_type),
        }
    }
}
