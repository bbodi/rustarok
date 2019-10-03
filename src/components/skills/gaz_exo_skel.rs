use crate::asset::SpriteResource;
use crate::common::ElapsedTime;
use crate::components::char::{ActionPlayMode, Percentage};
use crate::components::char::{
    CharAttributeModifier, CharAttributeModifierCollector, CharacterStateComponent,
};
use crate::components::controller::CharEntityId;
use crate::components::skills::basic_attack::{BasicAttack, WeaponType};
use crate::components::skills::skills::{
    FinishCast, FinishSimpleSkillCastComponent, SkillDef, SkillTargetType,
};
use crate::components::status::status::{
    ApplyStatusComponent, Status, StatusNature, StatusStackingResult, StatusUpdateResult,
};
use crate::components::StrEffectComponent;
use crate::configs::DevConfig;
use crate::consts::JobId;
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::{Sex, SystemVariables};
use specs::{Entities, LazyUpdate};

pub struct ExoSkeletonSkill;

pub const EXO_SKELETON_SKILL: &'static ExoSkeletonSkill = &ExoSkeletonSkill;

impl ExoSkeletonSkill {
    fn do_finish_cast(
        finish_cast: &FinishCast,
        entities: &Entities,
        updater: &LazyUpdate,
        dev_configs: &DevConfig,
        sys_vars: &mut SystemVariables,
    ) {
        let now = sys_vars.time;
        let configs = &dev_configs.skills.exoskeleton;
        let duration_seconds = configs.duration_seconds;
        sys_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_secondary_status(
                finish_cast.caster_entity_id,
                finish_cast.caster_entity_id,
                Box::new(ExoSkeletonStatus::new(
                    now,
                    duration_seconds,
                    configs.armor,
                    configs.attack_range,
                    configs.movement_speed,
                    configs.attack_damage,
                    configs.attack_speed,
                )),
            ));
    }
}

impl SkillDef for ExoSkeletonSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\cr_reflectshield.bmp"
    }

    fn finish_cast(&self, finish_cast_data: FinishCast, entities: &Entities, updater: &LazyUpdate) {
        updater.insert(
            entities.create(),
            FinishSimpleSkillCastComponent::new(finish_cast_data, ExoSkeletonSkill::do_finish_cast),
        )
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::NoTarget
    }
}

#[derive(Clone)]
struct ExoSkeletonStatus {
    started: ElapsedTime,
    until: ElapsedTime,
    armor: Percentage,
    attack_range: Percentage,
    movement_speed: Percentage,
    attack_damage: Percentage,
    attack_speed: Percentage,
}

impl ExoSkeletonStatus {
    fn new(
        now: ElapsedTime,
        duration: f32,
        armor: Percentage,
        attack_range: Percentage,
        movement_speed: Percentage,
        attack_damage: Percentage,
        attack_speed: Percentage,
    ) -> ExoSkeletonStatus {
        ExoSkeletonStatus {
            started: now,
            until: now.add_seconds(duration),
            armor,
            attack_range,
            movement_speed,
            attack_damage,
            attack_speed,
        }
    }
}

impl Status for ExoSkeletonStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn get_body_sprite<'a>(
        &self,
        sys_vars: &'a SystemVariables,
        job_id: JobId,
        sex: Sex,
    ) -> Option<&'a SpriteResource> {
        Some(&sys_vars.assets.sprites.exoskeleton)
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
        target_char.basic_attack = BasicAttack::Ranged {
            bullet_type: WeaponType::SilverBullet,
        };
        updater.insert(
            entities.create(),
            StrEffectComponent {
                effect_id: StrEffectType::Cart.into(),
                pos: target_char.pos(),
                start_time: sys_vars.time,
                die_at: None,
                play_mode: ActionPlayMode::Once,
            },
        );
    }

    fn calc_attribs(&self, modifiers: &mut CharAttributeModifierCollector) {
        modifiers.change_armor(
            CharAttributeModifier::AddPercentage(self.armor),
            self.started,
            self.until,
        );
        modifiers.change_walking_speed(
            CharAttributeModifier::AddPercentage(self.movement_speed),
            self.started,
            self.until,
        );
        modifiers.change_attack_range(
            CharAttributeModifier::AddPercentage(self.attack_range),
            self.started,
            self.until,
        );
        modifiers.change_attack_damage(
            CharAttributeModifier::IncreaseByPercentage(self.attack_damage),
            self.started,
            self.until,
        );
        modifiers.change_attack_speed(
            CharAttributeModifier::AddPercentage(self.attack_speed),
            self.started,
            self.until,
        );
    }

    fn update(
        &mut self,
        _self_char_id: CharEntityId,
        target_char: &mut CharacterStateComponent,
        _phyisic_world: &mut PhysicEngine,
        sys_vars: &mut SystemVariables,
        _entities: &Entities,
        _updater: &mut LazyUpdate,
    ) -> StatusUpdateResult {
        if self.until.has_already_passed(sys_vars.time) {
            target_char.basic_attack = BasicAttack::Melee;
            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }

    fn stack(&self, _other: &Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::DontAddTheNewStatus
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Supportive
    }

    // mount is disabled
    // splash damage
    // átváltozó és lebomló animáció
    // castolás szüntesse meg a mount statuszt
    // amikor meghal a combo animáciüó, 1 pillanatra felvillan
}