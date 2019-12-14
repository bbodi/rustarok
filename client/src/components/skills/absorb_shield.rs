use crate::components::char::ActionPlayMode;
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::components::status::status::{
    ApplyStatusComponent, StatusEnum, StatusStackingResult, StatusUpdateResult,
};
use crate::components::{
    HpModificationRequest, HpModificationResult, HpModificationResultType, HpModificationType,
};
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::RenderDesktopClientSystem;
use crate::systems::{AssetResources, CharEntityId, SystemVariables};
use rustarok_common::common::{ElapsedTime, Vec2};

pub struct AbsorbShieldSkill;

pub const ABSORB_SHIELD_SKILL: &'static AbsorbShieldSkill = &AbsorbShieldSkill;

impl SkillDef for AbsorbShieldSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\cr_reflectshield.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
        let now = sys_vars.time;
        let duration_seconds = ecs_world
            .read_resource::<DevConfig>()
            .skills
            .absorb_shield
            .duration_seconds;
        sys_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_status(
                params.caster_entity_id,
                params.target_entity.unwrap(),
                StatusEnum::AbsorbStatus(AbsorbStatus::new(
                    params.caster_entity_id,
                    now,
                    duration_seconds,
                )),
            ));
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}

#[derive(Clone, Debug)]
pub struct AbsorbStatus {
    pub caster_entity_id: CharEntityId,
    pub started: ElapsedTime,
    pub animation_started: ElapsedTime,
    pub until: ElapsedTime,
    pub absorbed_damage: u32,
}

impl AbsorbStatus {
    pub fn new(caster_entity_id: CharEntityId, now: ElapsedTime, duration: f32) -> AbsorbStatus {
        AbsorbStatus {
            caster_entity_id,
            started: now,
            animation_started: now.add_seconds(-1.9),
            until: now.add_seconds(duration),
            absorbed_damage: 0,
        }
    }

    pub fn update(
        &mut self,
        now: ElapsedTime,
        self_char_id: CharEntityId,
        hp_mod_requests: &mut Vec<HpModificationRequest>,
    ) -> StatusUpdateResult {
        if self.until.has_already_passed(now) {
            if self.absorbed_damage > 0 {
                hp_mod_requests.push(HpModificationRequest {
                    src_entity: self.caster_entity_id,
                    dst_entity: self_char_id,
                    typ: HpModificationType::Heal(self.absorbed_damage),
                });
            }
            StatusUpdateResult::RemoveIt
        } else {
            if self
                .animation_started
                .add_seconds(2.0)
                .has_already_passed(now)
            {
                self.animation_started = now.add_seconds(-1.9);
            }
            StatusUpdateResult::KeepIt
        }
    }

    pub fn hp_mod_is_calculated_but_not_applied_yet(
        &mut self,
        outcome: HpModificationResult,
    ) -> HpModificationResult {
        match outcome.typ {
            HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                HpModificationType::BasicDamage(value, _, _)
                | HpModificationType::SpellDamage(value, _)
                | HpModificationType::Poison(value) => {
                    self.absorbed_damage += value;
                    return outcome.absorbed();
                }
                HpModificationType::Heal(_) => return outcome,
            },
            HpModificationResultType::Blocked | HpModificationResultType::Absorbed => {
                return outcome
            }
        }
    }

    pub fn render(
        &self,
        now: ElapsedTime,
        assets: &AssetResources,
        char_pos: Vec2,
        render_commands: &mut RenderCommandCollector,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Ramadan,
            self.animation_started,
            &char_pos,
            assets,
            now,
            render_commands,
            ActionPlayMode::Repeat,
        );
    }

    pub fn stack(&self, _other: &StatusEnum) -> StatusStackingResult {
        // I think it should be overwritten only when the caster_entity_id is the same
        // otherwise other players should get the healed credits for their armors
        //        let other_absorb = unsafe { Statuses::hack_cast::<AbsorbStatus>(&other) };
        //        if other_absorb.until.is_later_than(self.until) {
        //            self.until = other_absorb.until;
        //            self.started = other_absorb.started;
        //            self.caster_entity_id = other_absorb.caster_entity_id;
        //            self.animation_started = other_absorb.animation_started;
        //        }
        StatusStackingResult::AddTheNewStatus
    }
}
