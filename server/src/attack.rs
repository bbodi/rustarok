use crate::{prepare_entity_id_for_sending, send_packet, OutPacketCollector, PacketTarget};
use rustarok_common::attack::{
    ApplyForceComponent, AreaAttackComponent, HpModificationRequest, HpModificationResult,
    HpModificationResultType, HpModificationType,
};
use rustarok_common::common::{EngineTime, GameTime, Local, SimulationTick};
use rustarok_common::components::char::{EntityId, LocalCharStateComp, StaticCharDataComponent};
use rustarok_common::config::CommonConfigs;
use rustarok_common::packets::from_server::FromServerPacket;
use specs::prelude::Join;
use specs::{ReadExpect, WriteExpect, WriteStorage};

pub struct AttackSystem;

impl AttackSystem {}

impl<'a> specs::System<'a> for AttackSystem {
    type SystemData = (
        specs::Entities<'a>,
        WriteStorage<'a, LocalCharStateComp<Local>>,
        WriteStorage<'a, StaticCharDataComponent>,
        ReadExpect<'a, EngineTime>,
        ReadExpect<'a, SimulationTick>,
        WriteExpect<'a, Vec<HpModificationRequest>>,
        WriteExpect<'a, Vec<AreaAttackComponent>>,
        WriteExpect<'a, Vec<ApplyForceComponent>>,
        WriteExpect<'a, CommonConfigs>,
        specs::Write<'a, specs::LazyUpdate>,
        WriteExpect<'a, OutPacketCollector>,
        // Option<specs::Write<'a, Vec<SystemEvent>>>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut auth_char_state_storage,
            mut static_char_data_storage,
            time,
            tick,
            mut hp_mod_requests,
            mut area_hp_mod_requests,
            mut pushes,
            dev_configs,
            mut updater,
            mut packet_sender,
        ): Self::SystemData,
    ) {
        {
            let mut new_hp_mod_reqs = area_hp_mod_requests
                .iter()
                .map(|area_hp_mod| {
                    AttackCalculation::apply_hp_mod_on_area(
                        &entities,
                        &auth_char_state_storage,
                        &area_hp_mod,
                    )
                })
                .flatten();
            hp_mod_requests.extend(new_hp_mod_reqs);
            area_hp_mod_requests.clear();
        }

        for hp_mod_req in hp_mod_requests.drain(..) {
            // TODO: char_state.cannot_control_until should be defined by this code
            // TODO: enemies can cause damages over a period of time, while they can die and be removed,
            // so src data (or an attack specific data structure) must be copied
            log::trace!("Process hp_mod_req {:?}", hp_mod_req);
            // copy them so hp_mod_req can be moved into the closure
            let attacker_id = hp_mod_req.src_entity;
            let attacked_id = hp_mod_req.dst_entity;

            let hp_mod_req_results = if let Some(src_char_state) =
                auth_char_state_storage.get(hp_mod_req.src_entity.into())
            {
                let src_auth_state = auth_char_state_storage
                    .get(hp_mod_req.src_entity.into())
                    .unwrap();
                if let Some(dst_char_state) =
                    auth_char_state_storage.get(hp_mod_req.dst_entity.into())
                {
                    let src_static_data = static_char_data_storage
                        .get(hp_mod_req.src_entity.into())
                        .unwrap();
                    let dst_static_data = static_char_data_storage
                        .get(hp_mod_req.dst_entity.into())
                        .unwrap();
                    let dst_auth_state = auth_char_state_storage
                        .get(hp_mod_req.dst_entity.into())
                        .unwrap();
                    let is_valid = dst_auth_state.state().is_alive()
                        && match hp_mod_req.typ {
                            HpModificationType::Heal(_) => {
                                src_static_data.team.can_support(dst_static_data.team)
                            }
                            _ => src_static_data.team.can_attack(dst_static_data.team),
                        };
                    if !is_valid {
                        log::warn!("Invalid hp_mod_req: {:?}", hp_mod_req);
                    }
                    if is_valid {
                        Some(AttackCalculation::apply_armor_calc(
                            src_char_state,
                            dst_char_state,
                            hp_mod_req,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            log::trace!("Attack outcomes: {:?}", hp_mod_req_results);

            for hp_mod_req_result in hp_mod_req_results.into_iter() {
                let (hp_mod_req_result, char_pos) = {
                    // let attacked_entity_state =
                    //     char_state_storage.get_mut(attacked_id.into()).unwrap();
                    // let hp_mod_req_result = AttackCalculation::alter_requests_by_attacked_statuses(
                    //     hp_mod_req_result,
                    //     attacked_entity_state,
                    //     &mut hp_mod_requests,
                    // );

                    let attacked_entity_auth_state =
                        auth_char_state_storage.get_mut(attacked_id.into()).unwrap();
                    AttackCalculation::apply_damage(
                        attacked_entity_auth_state,
                        &hp_mod_req_result,
                        time.now(),
                    );

                    // attacked_entity_state
                    //     .statuses
                    //     .hp_mod_has_been_applied_on_me(
                    //         attacked_id,
                    //         &hp_mod_req_result,
                    //         &mut hp_mod_requests,
                    //     );
                    // TODO: rather than this, create a common component which
                    // contains all the necessary info from which an other system will be able to
                    // generate the render and audio commands
                    let attacked_entity_auth_state =
                        auth_char_state_storage.get(attacked_id.into()).unwrap();
                    (hp_mod_req_result, attacked_entity_auth_state.pos())
                };

                {
                    // let attacker_entity_state =
                    //     char_state_storage.get_mut(attacker_id.into()).unwrap();
                    // attacker_entity_state
                    //     .statuses
                    //     .hp_mod_has_been_applied_on_enemy(
                    //         attacker_id,
                    //         &hp_mod_req_result,
                    //         &mut hp_mod_requests,
                    //     );
                }

                send_packet(
                    &mut packet_sender,
                    PacketTarget::Area(attacked_id),
                    FromServerPacket::Damage {
                        src_id: prepare_entity_id_for_sending(attacker_id),
                        dst_id: prepare_entity_id_for_sending(attacked_id),
                        typ: hp_mod_req_result.typ,
                    },
                );
                // AttackCalculation::make_sound(
                //     &entities,
                //     char_pos,
                //     attacked_id,
                //     &hp_mod_req_result,
                //     time.now(),
                //     &mut updater,
                //     &sys_vars.assets.sounds,
                // );
                // AttackCalculation::add_flying_damage_entity(
                //     &hp_mod_req_result,
                //     &entities,
                //     &mut updater,
                //     attacker_id,
                //     attacked_id,
                //     &char_pos,
                //     time.now(),
                // );

                // TODO2 events
                // if let Some(events) = &mut events {
                //     events.push(SystemEvent::HpModification {
                //         timestamp: *tick,
                //         src: attacker_id,
                //         dst: attacked_id,
                //         // TODO2
                //         //                        result: hp_mod_req_result,
                //     });
                // }
            }
        }
    }
}

struct AttackCalculation;

impl AttackCalculation {
    fn apply_damage(
        auth_char_comp: &mut LocalCharStateComp<Local>,
        outcome: &HpModificationResult,
        now: GameTime<Local>,
    ) {
        match outcome.typ {
            HpModificationResultType::Ok(hp_req_mod_type) => match hp_req_mod_type {
                HpModificationType::Heal(val) => {
                    auth_char_comp.hp = auth_char_comp
                        .calculated_attribs()
                        .max_hp
                        .min(auth_char_comp.hp + val as i32);
                }
                HpModificationType::BasicDamage(val, _display_type, _weapon_type) => {
                    auth_char_comp
                        .cannot_control_until
                        .run_at_least_until(now, 100);
                    auth_char_comp.set_receiving_damage();
                    auth_char_comp.hp -= val as i32;
                }
                HpModificationType::Poison(val) => {
                    auth_char_comp.hp -= val as i32;
                }
                HpModificationType::SpellDamage(val, _display_type) => {
                    auth_char_comp
                        .cannot_control_until
                        .run_at_least_until(now, 100);
                    auth_char_comp.set_receiving_damage();
                    auth_char_comp.hp -= val as i32;
                }
            },
            HpModificationResultType::Blocked => {}
            HpModificationResultType::Absorbed => {}
        }
    }

    pub fn apply_armor_calc(
        _src: &LocalCharStateComp<Local>,
        dst: &LocalCharStateComp<Local>,
        hp_mod_req: HpModificationRequest,
    ) -> HpModificationResult {
        return match hp_mod_req.typ {
            HpModificationType::SpellDamage(base_dmg, _damage_render_type) => {
                let dmg = dst
                    .calculated_attribs()
                    .armor
                    .subtract_me_from(base_dmg as i32);
                if dmg <= 0 {
                    hp_mod_req.blocked()
                } else {
                    hp_mod_req.allow(dmg as u32)
                }
            }
            HpModificationType::BasicDamage(base_dmg, _damage_render_type, _weapon_type) => {
                let atk = dbg!(base_dmg);
                let atk = dbg!(dst.calculated_attribs().armor).subtract_me_from(atk as i32);
                dbg!(atk);
                if atk <= 0 {
                    hp_mod_req.blocked()
                } else {
                    hp_mod_req.allow(atk as u32)
                }
            }
            HpModificationType::Heal(healed) => hp_mod_req.allow(healed),
            HpModificationType::Poison(dmg) => {
                let atk = dst.calculated_attribs().armor.subtract_me_from(dmg as i32);
                if atk <= 0 {
                    hp_mod_req.blocked()
                } else {
                    hp_mod_req.allow(dmg)
                }
            }
        };
    }

    pub fn apply_hp_mod_on_area(
        entities: &specs::Entities,
        auth_char_storage: &WriteStorage<LocalCharStateComp<Local>>,
        area_hpmod_req: &AreaAttackComponent,
    ) -> Vec<HpModificationRequest> {
        let mut result_attacks = vec![];
        for (target_entity_id, char_state) in (entities, auth_char_storage).join() {
            let target_entity_id = EntityId::new(target_entity_id);
            if area_hpmod_req
                .except
                .map(|it| it == target_entity_id)
                .unwrap_or(false)
            {
                continue;
            }

            // for optimized, shape-specific queries
            // ncollide2d::query::distance_internal::
            // TODO2
            //            let coll_result = ncollide2d::query::proximity(
            //                &area_hpmod_req.area_isom,
            //                &*area_hpmod_req.area_shape,
            //                &Isometry2::new(char_state.pos(), 0.0),
            //                &ncollide2d::shape::Ball::new(1.0),
            //                0.1,
            //            );
            //            if coll_result == Proximity::Intersecting {
            //                result_attacks.push(HpModificationRequest {
            //                    src_entity: area_hpmod_req.source_entity_id,
            //                    dst_entity: target_entity_id,
            //                    typ: area_hpmod_req.typ,
            //                });
            //            }
        }
        return result_attacks;
    }
}
