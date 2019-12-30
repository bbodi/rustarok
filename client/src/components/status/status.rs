use crate::components::char::{
    percentage, ActionPlayMode, CharAttributeModifier, CharAttributeModifierCollector,
    CharAttributes, CharacterStateComponent, Percentage,
};
use crate::components::skills::absorb_shield::AbsorbStatus;
use crate::components::skills::assa_blade_dash::AssaBladeDashStatus;
use crate::components::skills::assa_phase_prism::AssaPhasePrismStatus;
use crate::components::skills::falcon_carry::FalconCarryStatus;
use crate::components::skills::fire_bomb::FireBombStatus;
use crate::components::skills::gaz_exo_skel::ExoSkeletonStatus;
use crate::components::skills::wiz_pyroblast::PyroBlastTargetStatus;
use crate::components::status::attack_heal_status::AttackHealStatus;
use crate::components::status::attrib_mod::{ArmorModifierStatus, WalkingSpeedModifierStatus};
use crate::components::status::death_status::DeathStatus;
use crate::components::status::reflect_damage_status::ReflectDamageStatus;
use crate::components::status::sacrafice_status::SacrificeStatus;
use crate::components::status::stun::StunStatus;
use crate::components::{
    ApplyForceComponent, HpModificationRequest, HpModificationResult, HpModificationType,
};
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::grf::SpriteResource;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::RenderDesktopClientSystem;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::{AssetResources, SystemVariables};
use crate::ElapsedTime;
use nalgebra::Isometry2;
use rustarok_common::common::{EngineTime, Vec2};
use rustarok_common::components::char::{CharEntityId, JobId, Sex, StatusNature, Team};
use specs::{Entities, LazyUpdate};
use strum_macros::EnumCount;
use strum_macros::EnumDiscriminants;

#[derive(Debug)]
pub enum StatusStackingResult {
    DontAddTheNewStatus,
    AddTheNewStatus,
    Replace,
}

pub struct StatusUpdateParams<'a> {
    pub self_char_id: CharEntityId,
    pub target_char: &'a mut CharacterStateComponent,
    pub physics_world: &'a mut PhysicEngine,
    pub sys_vars: &'a mut SystemVariables,
    pub entities: &'a Entities<'a>,
    pub updater: &'a mut LazyUpdate,
    pub time: &'a EngineTime,
}

const NONSTACKABLE_STATUS_COUNT: usize = 6;

#[allow(variant_size_differences)]
#[derive(Clone, Debug, EnumCount, EnumDiscriminants)]
pub enum StatusEnum {
    MountedStatus {
        speedup: Percentage,
    },
    DeathStatus(DeathStatus),
    AssaBladeDashStatus(AssaBladeDashStatus),
    AssaPhasePrismStatus(AssaPhasePrismStatus),
    FalconCarryStatus(FalconCarryStatus),
    ExoSkeletonStatus(ExoSkeletonStatus),

    // stackable statuses
    AbsorbStatus(AbsorbStatus),
    FireBombStatus(FireBombStatus),
    PyroBlastTargetStatus(PyroBlastTargetStatus),
    #[allow(dead_code)]
    AttackHealStatus(AttackHealStatus), // TODO: stackable?
    ArmorModifierStatus(ArmorModifierStatus),
    WalkingSpeedModifierStatus(WalkingSpeedModifierStatus),
    #[allow(dead_code)]
    ReflectDamageStatus(ReflectDamageStatus), // TODO: stackable?,
    #[allow(dead_code)]
    SacrificeStatus(SacrificeStatus),
    PoisonStatus(PoisonStatus),
    StunStatus(StunStatus),
}

impl StatusEnum {
    // TODO: const fn
    fn can_target_move(&self) -> bool {
        match self {
            StatusEnum::AbsorbStatus(_)
            | StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::SacrificeStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::MountedStatus { .. } => true,
            StatusEnum::DeathStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::StunStatus(_)
            | StatusEnum::AssaPhasePrismStatus(_) => false,
        }
    }

    fn can_target_cast(&self) -> bool {
        match self {
            StatusEnum::AbsorbStatus(_)
            | StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::SacrificeStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::MountedStatus { .. } => true,
            StatusEnum::DeathStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::StunStatus(_)
            | StatusEnum::AssaPhasePrismStatus(_) => false,
        }
    }

    fn can_target_be_controlled(&self) -> bool {
        match self {
            StatusEnum::AbsorbStatus(_)
            | StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::SacrificeStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::StunStatus(_)
            | StatusEnum::MountedStatus { .. } => true,
            StatusEnum::DeathStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::AssaPhasePrismStatus(_) => false,
        }
    }

    // TODO: remove it
    fn typ(&self) -> StatusNature {
        match self {
            StatusEnum::AbsorbStatus(_) => StatusNature::Supportive,
            StatusEnum::MountedStatus { .. } => StatusNature::Supportive,
            StatusEnum::DeathStatus(_) => StatusNature::Supportive,
            StatusEnum::AssaBladeDashStatus(_) => StatusNature::Supportive,
            StatusEnum::AssaPhasePrismStatus(_) => StatusNature::Supportive,
            StatusEnum::FalconCarryStatus(_) => StatusNature::Supportive,
            StatusEnum::ExoSkeletonStatus(_) => StatusNature::Supportive,
            StatusEnum::FireBombStatus(_) => StatusNature::Supportive,
            StatusEnum::PyroBlastTargetStatus(_) => StatusNature::Supportive,
            StatusEnum::AttackHealStatus(_) => StatusNature::Supportive,
            StatusEnum::ArmorModifierStatus(_) => StatusNature::Supportive,
            StatusEnum::WalkingSpeedModifierStatus(_) => StatusNature::Supportive,
            StatusEnum::ReflectDamageStatus(_) => StatusNature::Supportive,
            StatusEnum::SacrificeStatus(_) => StatusNature::Supportive,
            StatusEnum::PoisonStatus(_) => StatusNature::Supportive,
            StatusEnum::StunStatus(_) => StatusNature::Supportive,
        }
    }

    pub fn get_body_sprite<'a>(
        &self,
        assets: &'a AssetResources,
        job_id: JobId,
        sex: Sex,
    ) -> Option<&'a SpriteResource> {
        match self {
            StatusEnum::AbsorbStatus(_) => None,
            StatusEnum::MountedStatus { .. } => {
                let sprites = &assets.sprites;
                sprites
                    .mounted_character_sprites
                    .get(&job_id)
                    .and_then(|it| it.get(sex as usize))
            }
            StatusEnum::DeathStatus(_) => None,
            StatusEnum::AssaBladeDashStatus(_) => None,
            StatusEnum::AssaPhasePrismStatus(_) => None,
            StatusEnum::FalconCarryStatus(_) => None,
            StatusEnum::ExoSkeletonStatus(_) => Some(&assets.sprites.exoskeleton),
            StatusEnum::FireBombStatus(_) => None,
            StatusEnum::PyroBlastTargetStatus(_) => None,
            StatusEnum::AttackHealStatus(_) => None,
            StatusEnum::ArmorModifierStatus(_) => None,
            StatusEnum::WalkingSpeedModifierStatus(_) => None,
            StatusEnum::ReflectDamageStatus(_) => None,
            StatusEnum::SacrificeStatus(_) => None,
            StatusEnum::PoisonStatus(_) => None,
            StatusEnum::StunStatus(_) => None,
        }
    }

    pub fn on_apply(
        &mut self,
        self_entity_id: CharEntityId,
        target_char: &mut CharacterStateComponent,
        entities: &Entities,
        updater: &mut LazyUpdate,
        assets: &AssetResources,
        time: &EngineTime,
        physics_world: &mut PhysicEngine,
    ) {
        // TODO2
        //        match self {
        //            StatusEnum::AssaBladeDashStatus(_) => {
        //                // allow to go through anything
        //                target_char.set_noncollidable(physics_world);
        //            }
        //            StatusEnum::FalconCarryStatus(_) => {
        //                target_char.set_noncollidable(physics_world);
        //                target_char.set_state(ClientCharState::StandBy, CharDir::South);
        //            }
        //            StatusEnum::ExoSkeletonStatus(status) => {
        //                status.on_apply(target_char, entities, updater, time.now())
        //            }
        //            StatusEnum::StunStatus(status) => status.on_apply(
        //                self_entity_id,
        //                target_char,
        //                entities,
        //                updater,
        //                assets,
        //                time.now(),
        //            ),
        //            StatusEnum::AbsorbStatus(_)
        //            | StatusEnum::MountedStatus { .. }
        //            | StatusEnum::DeathStatus(_)
        //            | StatusEnum::AssaPhasePrismStatus(_)
        //            | StatusEnum::FireBombStatus(_)
        //            | StatusEnum::PyroBlastTargetStatus(_)
        //            | StatusEnum::AttackHealStatus(_)
        //            | StatusEnum::ArmorModifierStatus(_)
        //            | StatusEnum::WalkingSpeedModifierStatus(_)
        //            | StatusEnum::ReflectDamageStatus(_)
        //            | StatusEnum::SacrificeStatus(_)
        //            | StatusEnum::PoisonStatus(_) => {}
        //        }
    }

    pub fn get_render_color(&self, now: ElapsedTime) -> [u8; 4] {
        match self {
            StatusEnum::AbsorbStatus(_)
            | StatusEnum::MountedStatus { .. }
            | StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::SacrificeStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::StunStatus(_) => [255, 255, 255, 255],
            StatusEnum::AssaBladeDashStatus(_) => [0, 0, 0, 0],
            StatusEnum::AssaPhasePrismStatus(_) => [0, 255, 255, 255],
            StatusEnum::PoisonStatus(_) => [128, 255, 128, 255],
            StatusEnum::DeathStatus(status) => [
                255,
                255,
                255,
                if status.is_npc {
                    255 - (now.percentage_between(status.started, status.remove_char_at) * 255.0)
                        as u8
                } else {
                    255
                },
            ],
        }
    }

    pub fn calc_attribs(&self, modifiers: &mut CharAttributeModifierCollector) {
        match self {
            StatusEnum::AbsorbStatus(_) => {}
            StatusEnum::MountedStatus { speedup } => {
                // it is applied directly on the base moving speed, since it is called first
                modifiers.change_walking_speed(
                    CharAttributeModifier::IncreaseByPercentage(*speedup),
                    ElapsedTime(0.0),
                    ElapsedTime(0.0),
                );
            }
            StatusEnum::ArmorModifierStatus(status) => {
                status.calc_attribs(modifiers);
            }
            StatusEnum::WalkingSpeedModifierStatus(status) => {
                status.calc_attribs(modifiers);
            }
            StatusEnum::ExoSkeletonStatus(status) => status.calc_attribs(modifiers),
            StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::SacrificeStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::AssaPhasePrismStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::DeathStatus(_)
            | StatusEnum::StunStatus(_) => {}
        }
    }

    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        match self {
            StatusEnum::AbsorbStatus(status) => status.update(
                params.time.now(),
                params.self_char_id,
                &mut params.sys_vars.hp_mod_requests,
            ),
            StatusEnum::AssaBladeDashStatus(status) => status.update(params),
            StatusEnum::AssaPhasePrismStatus(status) => status.update(params),
            StatusEnum::FalconCarryStatus(status) => status.update(params),
            StatusEnum::FireBombStatus(status) => status.update(params),
            StatusEnum::ExoSkeletonStatus(status) => status.update(params),
            StatusEnum::AttackHealStatus(status) => status.update(params),
            StatusEnum::ArmorModifierStatus(status) => status.update(params),
            StatusEnum::WalkingSpeedModifierStatus(status) => status.update(params),
            StatusEnum::ReflectDamageStatus(status) => status.update(params),
            StatusEnum::SacrificeStatus(status) => status.update(params),
            StatusEnum::PoisonStatus(status) => status.update(params),
            StatusEnum::StunStatus(status) => status.update(params),
            StatusEnum::MountedStatus { .. }
            | StatusEnum::DeathStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_) => StatusUpdateResult::KeepIt,
        }
    }

    pub fn hp_mod_is_calculated_but_not_applied_yet(
        &mut self,
        outcome: HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) -> HpModificationResult {
        match self {
            StatusEnum::AbsorbStatus(status) => {
                status.hp_mod_is_calculated_but_not_applied_yet(outcome)
            }
            StatusEnum::SacrificeStatus(status) => {
                status.hp_mod_is_calculated_but_not_applied_yet(outcome, hp_mod_reqs)
            }
            StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::AssaPhasePrismStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::DeathStatus(_)
            | StatusEnum::StunStatus(_)
            | StatusEnum::MountedStatus { .. } => outcome,
        }
    }

    pub fn hp_mod_has_been_applied_on_me(
        &mut self,
        self_id: CharEntityId,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
        match self {
            StatusEnum::ReflectDamageStatus(status) => {
                status.hp_mod_has_been_applied_on_me(self_id, outcome, hp_mod_reqs)
            }
            StatusEnum::SacrificeStatus(_)
            | StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::AssaPhasePrismStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::DeathStatus(_)
            | StatusEnum::StunStatus(_)
            | StatusEnum::MountedStatus { .. }
            | StatusEnum::AbsorbStatus(_) => {}
        }
    }

    pub fn hp_mod_has_been_applied_on_enemy(
        &mut self,
        self_id: CharEntityId,
        outcome: &HpModificationResult,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) {
        match self {
            StatusEnum::AttackHealStatus(status) => {
                status.hp_mod_has_been_applied_on_enemy(self_id, outcome, hp_mod_reqs)
            }
            StatusEnum::SacrificeStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::AssaPhasePrismStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::DeathStatus(_)
            | StatusEnum::StunStatus(_)
            | StatusEnum::MountedStatus { .. }
            | StatusEnum::AbsorbStatus(_) => {}
        }
    }

    pub fn allow_push(&self, _push: &ApplyForceComponent) -> bool {
        match self {
            StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::SacrificeStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::StunStatus(_)
            | StatusEnum::MountedStatus { .. } => true,
            StatusEnum::DeathStatus(_)
            | StatusEnum::AbsorbStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::FalconCarryStatus(_)
            | StatusEnum::AssaPhasePrismStatus(_) => false,
        }
    }

    pub fn stack(&mut self, other: &StatusEnum) -> StatusStackingResult {
        match self {
            StatusEnum::FireBombStatus(_)
            | StatusEnum::PyroBlastTargetStatus(_)
            | StatusEnum::AttackHealStatus(_)
            | StatusEnum::ArmorModifierStatus(_)
            | StatusEnum::WalkingSpeedModifierStatus(_)
            | StatusEnum::ReflectDamageStatus(_)
            | StatusEnum::PoisonStatus(_)
            | StatusEnum::StunStatus(_) => StatusStackingResult::AddTheNewStatus,
            StatusEnum::AbsorbStatus(status) => status.stack(other),
            StatusEnum::DeathStatus(_)
            | StatusEnum::SacrificeStatus(_)
            | StatusEnum::ExoSkeletonStatus(_)
            | StatusEnum::AssaBladeDashStatus(_)
            | StatusEnum::MountedStatus { .. }
            | StatusEnum::FalconCarryStatus(_) => StatusStackingResult::DontAddTheNewStatus,
            StatusEnum::AssaPhasePrismStatus(_) => StatusStackingResult::Replace,
        }
    }

    pub fn render(
        &self,
        char_state: &CharacterStateComponent,
        assets: &AssetResources,
        time: &EngineTime,
        render_commands: &mut RenderCommandCollector,
    ) {
        // TODO2
        //        let now = time.now();
        //        match self {
        //            StatusEnum::AbsorbStatus(status) => {
        //                status.render(now, assets, char_state.pos(), render_commands)
        //            }
        //            StatusEnum::MountedStatus { .. } => {}
        //            StatusEnum::ExoSkeletonStatus(_) => {}
        //            StatusEnum::FireBombStatus(status) => {
        //                status.render(char_state.pos(), now, assets, render_commands)
        //            }
        //            StatusEnum::PyroBlastTargetStatus(status) => {
        //                status.render(char_state.pos(), now, assets, render_commands)
        //            }
        //            StatusEnum::AttackHealStatus(status) => {
        //                status.render(char_state.pos(), now, assets, render_commands)
        //            }
        //            StatusEnum::ArmorModifierStatus(_) => {}
        //            StatusEnum::WalkingSpeedModifierStatus(_) => {}
        //            StatusEnum::ReflectDamageStatus(status) => {
        //                status.render(char_state.pos(), now, assets, render_commands)
        //            }
        //            StatusEnum::SacrificeStatus(status) => {
        //                status.render(char_state.pos(), now, assets, render_commands)
        //            }
        //            StatusEnum::PoisonStatus(status) => {
        //                status.render(char_state.pos(), now, assets, render_commands)
        //            }
        //            StatusEnum::StunStatus(status) => {
        //                status.render(char_state.pos(), now, assets, render_commands)
        //            }
        //            StatusEnum::DeathStatus(_) => {}
        //            StatusEnum::AssaBladeDashStatus(status) => {
        //                status.render(char_state, now, assets, render_commands)
        //            }
        //            StatusEnum::FalconCarryStatus(status) => status.render(assets, render_commands),
        //            StatusEnum::AssaPhasePrismStatus(_) => {}
        //        }
    }

    pub fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        match self {
            StatusEnum::AbsorbStatus(status) => Some((
                status.until,
                now.percentage_between(status.started, status.until),
            )),
            StatusEnum::MountedStatus { .. } => None,
            StatusEnum::ExoSkeletonStatus(status) => status.get_status_completion_percent(now),
            StatusEnum::FireBombStatus(status) => status.get_status_completion_percent(now),
            StatusEnum::PyroBlastTargetStatus(_) => None,
            StatusEnum::AttackHealStatus(status) => status.get_status_completion_percent(now),
            StatusEnum::ArmorModifierStatus(_) => None,
            StatusEnum::WalkingSpeedModifierStatus(_) => None,
            StatusEnum::ReflectDamageStatus(status) => status.get_status_completion_percent(now),
            StatusEnum::SacrificeStatus(status) => status.get_status_completion_percent(now),
            StatusEnum::PoisonStatus(status) => status.get_status_completion_percent(now),
            StatusEnum::StunStatus(status) => status.get_status_completion_percent(now),
            StatusEnum::DeathStatus(_) => None,
            StatusEnum::AssaBladeDashStatus(_) => None,
            StatusEnum::FalconCarryStatus(status) => status.get_status_completion_percent(now),
            StatusEnum::AssaPhasePrismStatus(_) => None,
        }
    }
}

// if you change the size, the update function has to be changed as well
const STATUS_ARRAY_SIZE: usize = 32;
pub struct Statuses {
    statuses: [Option<StatusEnum>; STATUS_ARRAY_SIZE],
    first_free_index: usize,
    cached_modifier_collector: CharAttributeModifierCollector,
}

unsafe impl Sync for Statuses {}

unsafe impl Send for Statuses {}

impl Statuses {
    pub fn new() -> Statuses {
        Statuses {
            statuses: Default::default(),
            first_free_index: NONSTACKABLE_STATUS_COUNT,
            cached_modifier_collector: CharAttributeModifierCollector::new(),
        }
    }

    pub fn get_statuses(&self) -> &[Option<StatusEnum>; STATUS_ARRAY_SIZE] {
        &self.statuses
    }

    pub fn can_move(&self) -> bool {
        let mut allow = true;
        for can_move in self
            .statuses
            .iter()
            .take(self.first_free_index)
            .map(|it| it.as_ref().map(|it| it.can_target_move()).unwrap_or(true))
        {
            allow &= can_move;
        }
        return allow;
    }

    pub fn can_cast(&self) -> bool {
        let mut allow = true;
        for can_cast in self
            .statuses
            .iter()
            .take(self.first_free_index)
            .map(|it| it.as_ref().map(|it| it.can_target_cast()).unwrap_or(true))
        {
            allow &= can_cast;
        }
        return allow;
    }

    pub fn can_be_controlled(&self) -> bool {
        let mut allow = true;
        for can_be_controlled in self.statuses.iter().take(self.first_free_index).map(|it| {
            it.as_ref()
                .map(|it| it.can_target_be_controlled())
                .unwrap_or(true)
        }) {
            allow &= can_be_controlled;
        }
        return allow;
    }

    pub fn allow_push(&mut self, push: &ApplyForceComponent) -> bool {
        let mut allow = true;
        for allow_push in self
            .statuses
            .iter_mut()
            .take(self.first_free_index)
            .map(|it| it.as_ref().map(|it| it.allow_push(push)).unwrap_or(true))
        {
            allow &= allow_push;
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
            status
                .as_mut()
                .unwrap()
                .hp_mod_has_been_applied_on_me(self_id, &outcome, hp_mod_reqs);
        }
    }

    pub fn update(
        &mut self,
        self_char_id: CharEntityId,
        char_state: &mut CharacterStateComponent,
        physics_world: &mut PhysicEngine,
        sys_vars: &mut SystemVariables,
        time: &EngineTime,
        entities: &Entities,
        updater: &mut LazyUpdate,
    ) -> u32 {
        let mut changed: u32 = 0;
        for (i, status) in self
            .statuses
            .iter_mut()
            .enumerate()
            .take(self.first_free_index)
            .filter(|(_i, it)| it.is_some())
        {
            let result = status.as_mut().unwrap().update(StatusUpdateParams {
                self_char_id,
                target_char: char_state,
                physics_world,
                sys_vars,
                entities,
                updater,
                time,
            });
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
        self.move_free_index();
    }

    pub fn render(
        &self,
        char_state: &CharacterStateComponent,
        assets: &AssetResources,
        time: &EngineTime,
        render_commands: &mut RenderCommandCollector,
    ) {
        let mut already_rendered: [bool; STATUSENUM_COUNT] = [false; STATUSENUM_COUNT];
        for status in self.statuses.iter().filter(|it| it.is_some()) {
            let status = status.as_ref().unwrap();
            let type_id = StatusEnumDiscriminants::from(status) as usize;
            if !already_rendered[type_id] {
                status.render(char_state, assets, time, render_commands);
                already_rendered[type_id] = true;
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
        assets: &'a AssetResources,
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
                .get_body_sprite(assets, job_id, sex);
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
        self.statuses[StatusEnumDiscriminants::MountedStatus as usize].is_some()
    }

    pub fn add(&mut self, new_status: StatusEnum) {
        log::debug!("Try to add status: {:?}", new_status);
        if self.first_free_index >= STATUS_ARRAY_SIZE {
            log::error!("There is no more space for new Status!");
            return;
        }

        let mut current_index = self.first_free_index;
        let mut stack_type = StatusStackingResult::AddTheNewStatus;
        let adding_status_type = StatusEnumDiscriminants::from(&new_status) as usize;
        for i in 0..self.first_free_index {
            if i < NONSTACKABLE_STATUS_COUNT {
                let target_slot_type = i;
                if target_slot_type == adding_status_type {
                    stack_type = self.statuses[i]
                        .as_mut()
                        .map(|it| it.stack(&new_status))
                        .unwrap_or(StatusStackingResult::Replace);
                    current_index = i;
                    log::trace!(
                        "Found NONSTACKABLE slot. stack_type: {:?}, index: {}",
                        stack_type,
                        current_index
                    );
                    break;
                }
            } else {
                if let Some(status) = self.statuses[i].as_mut() {
                    let target_slot_type = {
                        let s: &StatusEnum = status;
                        StatusEnumDiscriminants::from(s) as usize
                    };
                    if target_slot_type == adding_status_type {
                        stack_type = status.stack(&new_status);
                        current_index = i;
                        log::trace!(
                            "Found STACKABLE slot. stack_type: {:?}, index: {}",
                            stack_type,
                            current_index
                        );
                        break;
                    }
                }
            }
        }

        log::trace!("stack_type: {:?}, index: {}", stack_type, current_index);
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
        log::debug!("Remove all status");
        for status in self.statuses.iter_mut().take(self.first_free_index) {
            *status = None;
        }
        self.first_free_index = NONSTACKABLE_STATUS_COUNT;
    }

    pub fn remove_by_nature(&mut self, status_type: StatusNature) {
        log::debug!("remove_by_nature: {:?}", status_type);
        for status in self.statuses.iter_mut().take(self.first_free_index) {
            let should_remove = status
                .as_ref()
                .map(|it| it.typ() == status_type)
                .unwrap_or(false);
            if should_remove {
                *status = None;
            }
        }
        self.move_free_index();
    }

    pub fn remove(&mut self, discr: StatusEnumDiscriminants) {
        log::debug!("remove: {:?}", discr);
        for status in self.statuses.iter_mut().take(self.first_free_index) {
            let should_remove = status
                .as_ref()
                .map(|it| StatusEnumDiscriminants::from(it) == discr)
                .unwrap_or(false);
            if should_remove {
                *status = None;
            }
        }
        self.move_free_index();
    }

    fn move_free_index(&mut self) {
        while self.first_free_index > NONSTACKABLE_STATUS_COUNT
            && self.statuses[self.first_free_index - 1].is_none()
        {
            self.first_free_index -= 1;
        }
    }

    pub fn remove_if<F>(&mut self, predicate: F)
    where
        F: Fn(&StatusEnum) -> bool,
    {
        for status in self.statuses.iter_mut().take(self.first_free_index) {
            let should_remove = status.as_ref().map(|it| predicate(it)).unwrap_or(false);
            if should_remove {
                *status = None;
            }
        }
        self.move_free_index();
    }

    #[allow(dead_code)]
    pub fn get_status(&self, requested_type_id: StatusEnumDiscriminants) -> Option<&StatusEnum> {
        for status in self.statuses.iter().filter(|it| it.is_some()) {
            let status = status.as_ref().unwrap();
            let type_id: StatusEnumDiscriminants = status.into();
            if requested_type_id == type_id {
                return Some(status);
            }
        }
        return None;
    }

    #[allow(dead_code)]
    // for tests
    pub fn count(&self) -> usize {
        let secondary_status_count = self.first_free_index - NONSTACKABLE_STATUS_COUNT;
        let main_status_count = self
            .statuses
            .iter()
            .take(NONSTACKABLE_STATUS_COUNT)
            .filter(|it| it.is_some())
            .count();
        return main_status_count + secondary_status_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_stackable_statuses() {
        let mut statuses = Statuses::new();

        assert_eq!(statuses.first_free_index, NONSTACKABLE_STATUS_COUNT);
        assert!(statuses.statuses[0].is_none());
        statuses.add(StatusEnum::MountedStatus {
            speedup: percentage(0),
        });
        assert_eq!(statuses.first_free_index, NONSTACKABLE_STATUS_COUNT);
        assert!(statuses.statuses[0].is_some());
        statuses.add(StatusEnum::MountedStatus {
            speedup: percentage(0),
        });
        assert_eq!(statuses.first_free_index, NONSTACKABLE_STATUS_COUNT);
        assert!(statuses.statuses[0].is_some());

        statuses.remove(StatusEnumDiscriminants::MountedStatus);
        assert_eq!(statuses.first_free_index, NONSTACKABLE_STATUS_COUNT);
        assert!(statuses.statuses[0].is_none());
    }

    #[test]
    fn stackable_statuses() {
        let mut statuses = Statuses::new();

        assert_eq!(statuses.first_free_index, NONSTACKABLE_STATUS_COUNT);
        assert!(statuses.statuses[NONSTACKABLE_STATUS_COUNT].is_none());

        let status = StatusEnum::WalkingSpeedModifierStatus(WalkingSpeedModifierStatus {
            started: ElapsedTime(0.0),
            until: ElapsedTime(0.0),
            modifier: percentage(0),
        });

        statuses.add(status.clone());
        assert!(statuses.statuses[NONSTACKABLE_STATUS_COUNT].is_some());
        assert_eq!(statuses.first_free_index, NONSTACKABLE_STATUS_COUNT + 1);

        statuses.add(status.clone());
        assert_eq!(statuses.first_free_index, NONSTACKABLE_STATUS_COUNT + 2);
        assert!(statuses.statuses[NONSTACKABLE_STATUS_COUNT].is_some());
        assert!(statuses.statuses[NONSTACKABLE_STATUS_COUNT + 1].is_some());

        statuses.remove(StatusEnumDiscriminants::WalkingSpeedModifierStatus);
        assert_eq!(statuses.first_free_index, NONSTACKABLE_STATUS_COUNT);
        assert!(statuses.statuses[NONSTACKABLE_STATUS_COUNT].is_none());
        assert!(statuses.statuses[NONSTACKABLE_STATUS_COUNT + 1].is_none());
    }
}

pub enum StatusUpdateResult {
    RemoveIt,
    KeepIt,
}

#[derive(Clone, Debug)]
pub struct PoisonStatus {
    pub poison_caster_entity_id: CharEntityId,
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub next_damage_at: ElapsedTime,
    pub damage: u32,
}

impl PoisonStatus {
    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if self.until.has_already_passed(params.time.now()) {
            StatusUpdateResult::RemoveIt
        } else {
            if self.next_damage_at.has_already_passed(params.time.now()) {
                params.sys_vars.hp_mod_requests.push(HpModificationRequest {
                    src_entity: self.poison_caster_entity_id,
                    dst_entity: params.self_char_id,
                    typ: HpModificationType::Poison(30),
                });
                self.next_damage_at = params.time.now().add_seconds(1.0);
            }
            StatusUpdateResult::KeepIt
        }
    }

    pub fn render(
        &self,
        char_pos: Vec2,
        now: ElapsedTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Quagmire,
            self.started,
            &char_pos,
            assets,
            now,
            render_commands,
            ActionPlayMode::Repeat,
        );
    }

    pub fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }
}

pub struct ApplyStatusComponent {
    pub source_entity_id: CharEntityId,
    pub target_entity_id: CharEntityId,
    pub status: StatusEnum,
}

pub struct ApplyStatusInAreaComponent {
    pub source_entity_id: CharEntityId,
    pub status: StatusEnum,
    // TODO: it should not be a box. Predefine shapes
    pub area_shape: Box<dyn ncollide2d::shape::Shape<f32>>,
    pub area_isom: Isometry2<f32>,
    pub except: Option<CharEntityId>,
    pub nature: StatusNature,
    pub caster_team: Team,
}

pub enum RemoveStatusComponentPayload {
    RemovingStatusType(StatusNature),
    RemovingStatusDiscr(StatusEnumDiscriminants),
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
    pub fn from_status(
        source_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        m: StatusEnum,
    ) -> ApplyStatusComponent {
        ApplyStatusComponent {
            source_entity_id,
            target_entity_id,
            status: m,
        }
    }
}

impl RemoveStatusComponent {
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
