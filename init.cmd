

spawn_area poison 50 2 3 2000 500 251 -213
spawn_area absorb 50 2 3 2000 3000 255 -213
spawn_area firebomb 50 2 3 2000 2000 260 -213
spawn_area armor 70 2 3 2000 10000 265 -213
spawn_area armor -30 2 3 2000 10000 270 -213
spawn_area heal 50 2 3 500 0 273 -213
spawn_entity dummy_enemy left 1 217 -66
spawn_entity dummy_enemy left 1 219 -66
spawn_entity dummy_enemy left 1 221 -66
spawn_entity dummy_ally left 1 219 -71
// RIGHT TEAM GUARDS
// middle final 4 guards on lamps
spawn_entity guard right 1 246 -214 GEFFEN_MAGE_12 3.5
spawn_entity guard right 1 266 -214 GEFFEN_MAGE_12 3.5
spawn_entity guard right 1 246 -194 GEFFEN_MAGE_12 3.5
spawn_entity guard right 1 266 -194 GEFFEN_MAGE_12 3.5
// middle, middle 3 guards on bridge
spawn_entity guard right 1 195 -208 GEFFEN_MAGE_12 6
spawn_entity guard right 1 195 -204 GEFFEN_MAGE_12 6
spawn_entity guard right 1 195 -200 GEFFEN_MAGE_12 6
// top, guard alone on lamp
spawn_entity guard right 1 200 -326 GEFFEN_MAGE_12 3.5
// top, two guards on lamps
spawn_entity guard right 1 186 -299 GEFFEN_MAGE_12 3.5
spawn_entity guard right 1 200 -299 GEFFEN_MAGE_12 3.5
// LEFT TEAM GUARDS
// middle final 4 guards on lamps
spawn_entity guard left 1 66 -214 GEFFEN_MAGE_9 3.5
spawn_entity guard left 1 48 -214 GEFFEN_MAGE_9 3.5
spawn_entity guard left 1 66 -194 GEFFEN_MAGE_9 3.5
spawn_entity guard left 1 48 -194 GEFFEN_MAGE_9 3.5
// middle, middle 3 guards on bridge
spawn_entity guard left 1 117 -208 GEFFEN_MAGE_9 6
spawn_entity guard left 1 117 -204 GEFFEN_MAGE_9 6
spawn_entity guard left 1 117 -200 GEFFEN_MAGE_9 6
// top, guard alone on lamp
spawn_entity guard left 1 112 -326 GEFFEN_MAGE_9 3.5
// top, two guards on lamps
spawn_entity guard left 1 112 -299 GEFFEN_MAGE_9 3.5
spawn_entity guard left 1 126 -299 GEFFEN_MAGE_9 3.5

// syntax: bind_key key_specifier script
// - key_specifier must not contain any whitespace character
// - maximum combination of keys is 3
// bind_key Q spell_q
// bind_key W spell_w
// bind_key E spell_e
// bind_key R spell_r
// bind_key D spell_d
// bind_key Y mounting
// bind_key A attack_move_target_selection
// bind_key Num1 spell_1
// bind_key Num2 spell_2
// bind_key Num3 spell_3
// bind_key Num4 spell_4

// bind_key right_mouse_btn cancel_targeting
// bind_key left_mouse_btn choose_target
// smart command: if clicks on enemy, attack it, if on ground, move
// move
// attack_move
// bind_key right_mouse_btn smart_command
// bind_key left_mouse_btn move_minimap_camera

bind_key alt+Num1 toggle_console