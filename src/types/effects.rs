use crate::util::timed_events::{ DelayedEvent, RepeatedEvent };
use crate::util::access::{ self, EntityAccessor };
use crate::traits::Entity;
use crate::*;

use self::EffectType::*;

use rand::distributions::{ Weighted, WeightedChoice, Sample };
use rand::{ Rng, thread_rng };

#[derive(Clone, Eq, PartialEq)]
pub enum EffectType
{
    Permanent,
    Temporary(u64),
    Repeat(u64, u64)
}

#[derive(Clone)]
pub struct Effect
{
    pub name: &'static str,
    pub level: u32,
    pub effect_type: EffectType,
    pub health: i32,
    pub break_health_cap: bool,
    pub max_health: i32,
    pub base_damage: i32,
    pub attack_speed: i32,
    pub break_attack_cap: bool,
    pub item_speed: i32,
    pub break_item_cap: bool,
    pub money: i32,
}

impl Default for Effect
{
    fn default() -> Effect
    {
        Effect
        {
            name: "Unnamed Potion",
            level: 0,
            effect_type: Permanent,
            health: 0,
            break_health_cap: false,
            max_health: 0,
            base_damage: 0,
            attack_speed: 0,
            break_attack_cap: false,
            item_speed: 0,
            break_item_cap: false,
            money: 0
        }
    }
}

/**
 * These are not IDs.
 */
const HEALTH: i32 = 1;
const DAMAGE: i32 = 2;
const ATK_SPEED: i32 = 3;
const ITEM_SPEED: i32 = 4;
const MONEY: i32 = 5;
const ABSORPTION: i32 = 6;
const STRENGTH: i32 = 7;
const ATK_SWIFTNESS: i32 = 8;
const ITEM_SWIFTNESS: i32 = 9;
const GAMBLING: i32 = 10;

const MAX_EFFECT_DURATION: u64 = 600_000; // 10 minutes

impl Effect
{
//    pub fn get_leveled_health(town_num: usize) -> Effect
//    {
//
//    }

    /**
     * Max level: 10
     * 5 + (5 hp per level)
     */
    pub fn leveled_health(mut level: u32) -> Effect
    {
        if level > 10 { level = 10; }

        Effect
        {
            name: "Healing",
            health: 5 + (level as i32 * 5),
            level,
            ..Self::default()
        }
    }

    pub fn generic_health(amount: i32) -> Effect
    {
        Effect
        {
            name: "Healing",
            health: amount,
            ..Self::default()
        }
    }

//    pub fn get_leveled_damage(town_num: usize) -> Effect
//    {
//
//    }

    /**
     * Max level: 20
     * -5 hp per level
     */
    pub fn leveled_damage(mut level: u32) -> Effect
    {
        if level > 20 { level = 20; }

        Effect
        {
            name: "Harming",
            health: -5 * level as i32,
            level,
            ..Self::default()
        }
    }

    pub fn generic_damage(amount: i32) -> Effect
    {
        Effect
        {
            name: "Harming",
            health: -1 * amount,
            ..Self::default()
        }
    }

    /**
     * +1 level per 3 * town_num
     * +1 variability per 5 * town_num
     */
    pub fn get_leveled_absorption(town_num: usize) -> Effect
    {
        let base_level = (town_num / 3) + 1; // Start at level = 1
        let variability = town_num / 5; // Start at variability = 0;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_absorption(level as u32)
    }

    /**
     * Max level: 10
     * 5 hp per level
     * 5 max hp per level
     * 1 minute per 2 levels
     */
    pub fn leveled_absorption(mut level: u32) -> Effect
    {
        if level > 10 { level = 10; }

        let value = level as i32 * 5;
        let duration = 60_000 * ((level as i32 / 2) + 1);

        Effect
        {
            name: "Absorption",
            max_health: value,
            health: value,
            level,
            effect_type: Temporary(duration as u64),
            ..Self::default()
        }
    }

    /**
     * +1 level per 3 * town_num
     * +1 variability per 5 * town_num
     */
    pub fn get_leveled_fragile_skin(town_num: usize) -> Effect
    {
        let base_level = (town_num / 3) + 1; // Start at level = 1
        let variability = town_num / 5; // Start at variability = 0;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_fragile_skin(level as u32)
    }

    /**
     * Max level: 5
     * -5 hp per level
     * -5 max hp per level
     * 1 minute per 3 levels
     */
    pub fn leveled_fragile_skin(mut level: u32) -> Effect
    {
        if level > 10 { level = 10; }

        let value = level as i32 * -5;
        let duration = 60_000 * ((level as u64 / 3) + 1);

        Effect
        {
            name: "Absorption",
            max_health: value,
            health: value,
            level,
            effect_type: Temporary(duration),
            ..Self::default()
        }
    }

    /**
     * +1 level per 3 * town_num
     * +1 variability per 4 * town_num
     */
    pub fn get_leveled_strength(town_num: usize) -> Effect
    {
        let base_level = (town_num / 3) + 1; // Start at level = 1
        let variability = town_num / 4; // Start at variability = 0;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_strength(level as u32)
    }

    /**
     * Max level: None
     * 5 damage per level
     * 1 minute per 2 levels
     */
    pub fn leveled_strength(level: u32) -> Effect
    {
        let value = level as i32 * 5;
        let mut duration = 60_000 * ((level as u64 / 2) + 1);

        if duration > MAX_EFFECT_DURATION { duration = MAX_EFFECT_DURATION; }

        Effect
        {
            name: "Strength",
            base_damage: value,
            level,
            effect_type: Temporary(duration),
            ..Self::default()
        }
    }

    /**
     * +1 level per 3 * town_num
     * +1 variability per 4 * town_num
     */
    pub fn get_leveled_weakness(town_num: usize) -> Effect
    {
        let base_level = (town_num / 3) + 1; // Start at level = 1
        let variability = town_num / 4; // Start at variability = 0;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_weakness(level as u32)
    }

    /**
     * Max level: None
     * -5 damage per level
     * 1 minute per 3 levels
     */
    pub fn leveled_weakness(level: u32) -> Effect
    {
        let value = level as i32 * -5;
        let mut duration = 60_000 * ((level as u64 / 3) + 1);

        if duration > MAX_EFFECT_DURATION { duration = MAX_EFFECT_DURATION; }

        Effect
        {
            name: "Strength",
            base_damage: value,
            level,
            effect_type: Temporary(duration),
            ..Self::default()
        }
    }

    /**
     * +1 level per 2 towns.
     * +1 variability per 2 towns.
     */
    pub fn get_leveled_atk_swiftness(town_num: usize) -> Effect
    {
        let variability = town_num / 2;
        let base_level = variability + 1;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_atk_swiftness(level as u32)
    }

    /**
     * Max level: None --Not sure
     * -0.5 seconds delay per level
     * 1 minute per 2 levels
     */
    pub fn leveled_atk_swiftness(level: u32) -> Effect
    {
        let value = level as i32 * -500;
        let mut duration = 60_000 * ((level as u64 / 3) + 1);

        if duration > MAX_EFFECT_DURATION { duration = MAX_EFFECT_DURATION; }

        Effect
        {
            name: "Attack Swiftness",
            attack_speed: value,
            level,
            effect_type: Temporary(duration),
            ..Self::default()
        }
    }

    /**
     * +1 level per 2 towns.
     * +1 variability per 2 towns.
     */
    pub fn get_leveled_atk_slowness(town_num: usize) -> Effect
    {
        let variability = town_num / 2;
        let base_level = variability + 1;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_atk_slowness(level as u32)
    }

    /**
     * Max level: 15
     * 0.5 seconds delay per level
     * 1 minute per 3 levels
     */
    pub fn leveled_atk_slowness(mut level: u32) -> Effect
    {
        if level > 15 { level = 15; }

        let value = level as i32 * 500;
        let duration = 60_000 * ((level as i64 / 3) + 1);

        Effect
        {
            name: "Attack Slowness",
            attack_speed: value,
            level,
            effect_type: Temporary(duration as u64),
            ..Self::default()
        }
    }

    /**
     * +1 level per 2 towns.
     * +1 variability per 2 towns.
     */
    pub fn get_leveled_item_swiftness(town_num: usize) -> Effect
    {
        let variability = town_num / 2;
        let base_level = variability + 1;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_item_swiftness(level as u32)
    }

    /**
     * Max level: None --Not sure
     * -0.5 seconds delay per level
     * 1 minute per 2 levels
     */
    pub fn leveled_item_swiftness(level: u32) -> Effect
    {
        let value = level as i32 * -500;
        let mut duration = 60_000 * ((level as u64 / 2) + 1);

        if duration > MAX_EFFECT_DURATION { duration = MAX_EFFECT_DURATION; }

        Effect
        {
            name: "Item Swiftness",
            item_speed: value,
            level,
            effect_type: Temporary(duration),
            ..Self::default()
        }
    }

    /**
     * +1 level per 2 towns.
     * +1 variability per 2 towns.
     */
    pub fn get_leveled_item_slowness(town_num: usize) -> Effect
    {
        let variability = town_num / 2;
        let base_level = variability + 1;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_item_slowness(level as u32)
    }

    /**
     * Max level: 15
     * 0.5 seconds delay per level
     * 1 minute per 3 levels
     */
    pub fn leveled_item_slowness(mut level: u32) -> Effect
    {
        if level > 15 { level = 15; }

        let value = level as i32 * 500;
        let duration = 60_000 * ((level as i64 / 3) + 1);

        Effect
        {
            name: "Item Slowness",
            item_speed: value,
            level,
            effect_type: Temporary(duration as u64),
            ..Self::default()
        }
    }

    /**
     * +1 level per 7 * town_num
     * +1 variability per (7 * town_num) + 1
     */
    pub fn get_leveled_gambling(town_num: usize) -> Effect
    {
        let base_level = (town_num / 7) + 1; // Start at level = 1
        let variability = base_level; // Start at variability = 0;

        let level = thread_rng().gen_range(base_level - variability, base_level + variability + 1);

        Self::leveled_gambling(level as u32)
    }

    /**
     * Max level: 5,
     * 750g per level
     * 15 + (10 seconds per level)
     */
    pub fn leveled_gambling(mut level: u32) -> Effect
    {
        if level > 5 { level = 5; }
        else if level < 1 { level = 1; }

        let value = level as i32 * 750;
        let duration = 15_000 + (10_000 * level as i64);

        Effect
        {
            name: "Gambling",
            money: value,
            level,
            effect_type: Temporary(duration as u64),
            ..Self::default()
        }
    }

    pub fn random_permanent_blessing() -> Effect
    {
        let mut blessings =
        [
            Weighted { weight: 1, item: HEALTH },
            Weighted { weight: 3, item: DAMAGE },
            Weighted { weight: 4, item: ATK_SPEED },
            Weighted { weight: 4, item: ITEM_SPEED },
            Weighted { weight: 4, item: MONEY }
        ];

        let result = WeightedChoice::new(&mut blessings).sample(&mut thread_rng());

        match result
        {
            HEALTH => Self::standard_health_up(),
            DAMAGE => Self::standard_damage_up(),
            ATK_SPEED => Self::standard_atk_speed_up(),
            ITEM_SPEED => Self::standard_item_speed_up(),
            MONEY => Self::standard_money_up(),
            _ => Self::default()
        }
    }

    pub fn random_permanent_curse() -> Effect
    {
        let mut blessings =
        [
            Weighted { weight: 1, item: HEALTH },
            Weighted { weight: 2, item: DAMAGE },
            Weighted { weight: 3, item: ATK_SPEED },
            Weighted { weight: 3, item: ITEM_SPEED },
            Weighted { weight: 3, item: MONEY }
        ];

        let result = WeightedChoice::new(&mut blessings).sample(&mut thread_rng());

        match result
        {
            HEALTH => Self::standard_health_down(),
            DAMAGE => Self::standard_damage_down(),
            ATK_SPEED => Self::standard_atk_speed_down(),
            ITEM_SPEED => Self::standard_item_speed_down(),
            MONEY => Self::standard_money_down(),
            _ => Self::default()
        }
    }

    pub fn normal_altar_effect() -> (Effect, Effect)
    {
        let mut effects =
        [
            Weighted { weight: 1, item: HEALTH },
            Weighted { weight: 2, item: DAMAGE },
            Weighted { weight: 3, item: ATK_SPEED },
            Weighted { weight: 3, item: ITEM_SPEED },
            Weighted { weight: 3, item: MONEY }
        ];

        let mut chooser = WeightedChoice::new(&mut effects);
        let blessing = chooser.sample(&mut thread_rng());
        let mut curse = chooser.sample(&mut thread_rng());

        while blessing == curse // Ensure that the effects are not the same.
        {
            curse = chooser.sample(&mut thread_rng());
        }

        let blessing = match blessing
        {
            HEALTH => Self::generic_health_up(5),
            DAMAGE => Self::generic_damage_up(5, 11),
            ATK_SPEED => Self::generic_atk_speed_up(250, 500),
            ITEM_SPEED => Self::generic_item_speed_up(250, 500),
            MONEY => Self::generic_money_up(350, 1000),
            _ => Self::default()
        };
        let curse = match curse
        {
            HEALTH => Self::generic_health_down(3),
            DAMAGE => Self::generic_damage_down(2, 10),
            ATK_SPEED => Self::generic_atk_speed_down(100, 450),
            ITEM_SPEED => Self::generic_item_speed_down(100, 450),
            MONEY => Self::generic_money_down(100, 900),
            _ => Self::default()
        };
        (blessing, curse)
    }

    /**
     * Different from random_permanent_blessing()
     * in that it's rarer / better.
     */
    pub fn positive_altar_effect() -> Effect
    {
        let mut blessings =
        [
            Weighted { weight: 1, item: HEALTH },
            Weighted { weight: 3, item: DAMAGE },
            Weighted { weight: 4, item: ATK_SPEED },
            Weighted { weight: 4, item: ITEM_SPEED },
            Weighted { weight: 4, item: MONEY }
        ];

        let result = WeightedChoice::new(&mut blessings).sample(&mut thread_rng());

        match result
        {
            HEALTH => Self::generic_health_up(8),
            DAMAGE => Self::generic_damage_up(10, 16),
            ATK_SPEED => Self::generic_atk_speed_up(450, 900),
            ITEM_SPEED => Self::generic_item_speed_up(450, 900),
            MONEY => Self::generic_money_up(550, 1500),
            _ => Self::default()
        }
    }

    pub fn get_fountain_effect(town_num: usize) -> Effect
    {
        let result = *choose(
            &[ABSORPTION, STRENGTH, ATK_SWIFTNESS, ITEM_SWIFTNESS, GAMBLING]
        );

        match result
        {
            ABSORPTION => Self::get_leveled_absorption(town_num),
            STRENGTH => Self::get_leveled_strength(town_num),
            ATK_SWIFTNESS => Self::get_leveled_atk_swiftness(town_num),
            ITEM_SWIFTNESS => Self::get_leveled_item_swiftness(town_num),
            GAMBLING => Self::get_leveled_gambling(town_num),
            _ => Self::default()
        }
    }

    pub fn standard_health_up() -> Effect
    {
        Self::generic_health_up(5)
    }

    pub fn generic_health_up(amount: i32) -> Effect
    {
        Effect
        {
            name: "Health Up",
            max_health: amount,
            ..Self::default()
        }
    }

    pub fn standard_health_down() -> Effect
    {
        Self::generic_health_down(5)
    }

    pub fn generic_health_down(amount: i32) -> Effect
    {
        Effect
        {
            name: "Health Down",
            max_health: -1 * amount,
            ..Self::default()
        }
    }

    pub fn standard_damage_up() -> Effect
    {
        Self::generic_damage_up(0, 11)
    }

    pub fn generic_damage_up(min: i32, max: i32) -> Effect
    {
        Effect
        {
            name: "Damage Up",
            base_damage: thread_rng().gen_range(min, max),
            ..Self::default()
        }
    }

    pub fn standard_damage_down() -> Effect
    {
        Self::generic_damage_down(0, 11)
    }

    pub fn generic_damage_down(min: i32, max: i32) -> Effect
    {
        Effect
        {
            name: "Damage Down",
            base_damage: thread_rng().gen_range(max * -1, min * -1),
            ..Self::default()
        }
    }

    pub fn standard_atk_speed_up() -> Effect
    {
        Self::generic_atk_speed_up(0, 500)
    }

    pub fn generic_atk_speed_up(min: i32, max: i32) -> Effect
    {
        Effect
        {
            name: "Atk Speed Up",
            attack_speed: thread_rng().gen_range(max * -1, min * -1),
            ..Self::default()
        }
    }

    pub fn standard_atk_speed_down() -> Effect
    {
        Self::generic_atk_speed_down(0, 500)
    }

    pub fn generic_atk_speed_down(min: i32, max: i32) -> Effect
    {
        Effect
        {
            name: "Atk Speed Down",
            attack_speed: thread_rng().gen_range(min, max),
            ..Self::default()
        }
    }

    pub fn standard_item_speed_up() -> Effect
    {
        Self::generic_item_speed_up(0, 500)
    }

    pub fn generic_item_speed_up(min: i32, max: i32) -> Effect
    {
        Effect
        {
            name: "Item Speed Up",
            item_speed: thread_rng().gen_range(max * -1, min * -1),
            ..Self::default()
        }
    }

    pub fn standard_item_speed_down() -> Effect
    {
        Self::generic_item_speed_down(0, 500)
    }

    pub fn generic_item_speed_down(min: i32, max: i32) -> Effect
    {
        Effect
        {
            name: "Item Speed Down",
            item_speed: thread_rng().gen_range(min, max),
            ..Self::default()
        }
    }

    pub fn standard_money_up() -> Effect
    {
        Self::generic_money_up(250, 1000)
    }

    pub fn generic_money_up(min: i32, max: i32) -> Effect
    {
        Effect
        {
            name: "Money Up",
            money: thread_rng().gen_range(min, max),
            ..Self::default()
        }
    }

    pub fn standard_money_down() -> Effect
    {
        Self::generic_money_down(250, 1000)
    }

    pub fn generic_money_down(min: i32, max: i32) -> Effect
    {
        Effect
        {
            name: "Money Down",
            money: thread_rng().gen_range(max * -1, min * -1),
            ..Self::default()
        }
    }

    pub fn apply(&self, to_entity: &Entity)
    {
        let generated = self.generate(to_entity);
        let potion_ref: &'static str = self.name;
        let accessor= to_entity.get_accessor();

        /**
         * They store the effect object, but not the actual effect.
         * generated() takes care of that.
         */
        to_entity.give_effect(self.clone());

        match self.effect_type
        {
            Permanent =>
            {
                generated();

                if to_entity.get_type() == "player"
                {
                    send_short_message(to_entity.get_id(), &format!("You got a permanent {} effect.", self.name));
                }

                to_entity.remove_effect(potion_ref);
            },
            Temporary(duration) =>
            {
                generated();

                DelayedEvent::new(duration, None, Some(to_entity.get_id()),
                Some(self.name.to_string()), move ||
                {
                    access::entity(accessor, |entity |
                    {
                        entity.remove_effect(potion_ref);
                    });
                });
            },
            Repeat(interval, duration) =>
            {
                RepeatedEvent::new(interval, duration, None, Some(to_entity.get_id()),
                    Some(self.name.to_string()), move ||
                {
                    generated()
                });

                DelayedEvent::no_flags(duration, move ||
                {
                    access::entity(accessor, |entity |
                    {
                        entity.remove_effect(potion_ref);
                    });
                });
            }
        }
    }

    pub fn remove(&self, from_entity: &Entity)
    {
        if let Temporary(_len) = self.effect_type
        {
            let opposite = self.get_opposite_effect();

            if opposite.max_health != 0
            {
                opposite.update_max_health(from_entity);
            }
            if opposite.health != 0
            {
                opposite.update_health(from_entity);
            }
            if opposite.attack_speed != 0
            {
                opposite.update_atk_speed(from_entity);
            }
            if opposite.item_speed != 0
            {
                opposite.update_item_speed(from_entity);
            }
            if opposite.base_damage != 0
            {
                opposite.update_base_damage(from_entity);
            }
            if opposite.money != 0
            {
                opposite.update_money(from_entity);
            }
            from_entity.update_health_bar();

            if from_entity.get_type() == "player"
            {
                send_short_message(from_entity.get_id(), &format!("{} effect wore off.", self.name));
            }
        }
    }

    pub fn get_opposite_effect(&self) -> Effect
    {
        Effect
        {
            health: self.health * -1,
            max_health: self.max_health * -1,
            base_damage: self.base_damage * -1,
            attack_speed: self.attack_speed * -1,
            item_speed: self.item_speed * -1,
            money: self.money * -1,
            ..Self::default()
        }
    }

    fn generate(&self, entity: &Entity) -> Box<'static + Fn() -> bool>
    {
        match self.effect_type
        {
            Temporary(_dur) if entity.get_type() == "player" =>
            {
                updatable_effect(self.name, entity.get_accessor())
            }
            _ => standard_effect(self.clone(), entity.get_accessor())
        }
    }

    fn update_health(&self, entity: &Entity)
    {
        if self.break_health_cap
        {
            let current = entity.get_health();
            let mut new = current as i32 + self.health;
            if new < 0 { new = 0; } // Prevent the cast from returning a very large number.
            entity.set_health(new as u32);
        }
        else { entity.add_health(self.health); }
    }

    fn mut_update_health(&mut self, entity: &Entity)
    {
        let old = entity.get_health();
        let mut new;

        if self.break_health_cap
        {
            new = old as i32 + self.health;
            if new < 0 { new = 0; }
            entity.set_health(new as u32);
        }
        else
        {
            entity.add_health(self.health);
            new = entity.get_health() as i32;
        }

        let difference = new - old as i32;
        self.health = difference;
    }

    /**
     * Potential clarity improvement:
     * Don't check this operation here.
     */
    fn update_max_health(&self, entity: &Entity)
    {
        let mut updated = entity.get_max_health() as i32 + self.max_health;
        if updated < 5 { updated = 5; }

        entity.set_max_health(updated as u32);
    }

    fn mut_update_max_health(&mut self, entity: &Entity)
    {
        let old = entity.get_max_health();

        let mut new = entity.get_max_health() as i32 + self.max_health;
        if new < 5 { new = 5; }

        entity.set_max_health(new as u32);

        let difference = new - old as i32;
        self.max_health = difference;
    }

    fn update_atk_speed(&self, entity: &Entity)
    {
        if self.break_attack_cap
        {
            let current = entity.get_attack_speed();
            entity.set_attack_speed(current + self.attack_speed);
        }
        else { entity.add_attack_speed(self.attack_speed); }
    }

    fn mut_update_atk_speed(&mut self, entity: &Entity)
    {
        let old = entity.get_attack_speed();

        if self.break_attack_cap
        {
            entity.set_attack_speed(old + self.attack_speed);
        }
        else { entity.add_attack_speed(self.attack_speed); }

        let new = entity.get_attack_speed();

        let difference = new - old;
        self.attack_speed = difference;
    }

    fn update_item_speed(&self, entity: &Entity)
    {
        if self.break_item_cap
        {
            let current = entity.get_item_speed();
            entity.set_item_speed(current + self.item_speed)
        }
        else { entity.add_item_speed(self.item_speed) }
    }

    fn mut_update_item_speed(&mut self, entity: &Entity)
    {
        let old = entity.get_item_speed();

        if self.break_item_cap
        {
            entity.set_item_speed(old + self.item_speed);
        }
        else { entity.add_item_speed(self.item_speed); }

        let new = entity.get_item_speed();

        let difference = new - old;
        self.item_speed = difference;
    }

    fn update_base_damage(&self, entity: &Entity)
    {
        let current = entity.get_base_damage();
        let mut new = current as i32 + self.base_damage;
        if new < 0 { new = 0; } // Prevent the cast from returning a very large number.
        entity.set_base_damage(new as u32);
    }

    fn mut_update_base_damage(&mut self, entity: &Entity)
    {
        let old = entity.get_base_damage();
        let mut new = old as i32 + self.base_damage;
        if new < 0 { new = 0; } // Prevent the cast from returning a very large number.
        entity.set_base_damage(new as u32);

        let difference = new - old as i32;
        self.base_damage = difference;
    }

    fn update_money(&self, entity: &Entity)
    {
        if self.money < 0
        {
            entity.take_money(self.money.abs() as u32);
        }
        else { entity.give_money(self.money as u32); }
    }

    fn mut_update_money(&mut self, entity: &Entity)
    {
        let old = entity.get_money();

        if self.money < 0
        {
            entity.take_money(self.money.abs() as u32);
        }
        else { entity.give_money(self.money as u32); }

        let new = entity.get_money();

        let difference = new as i32 - old as i32;
        self.money = difference;
    }
}

fn standard_effect(effect: Effect, accessor: EntityAccessor) -> Box<'static + Fn() -> bool>
{
    Box::new(move ||
    {
        match access::entity(accessor, |entity |
        {
            if entity.has_effect(effect.name)
            {
                if effect.max_health != 0
                {
                    effect.update_max_health(entity);
                }
                if effect.health != 0
                {
                    effect.update_health(entity);
                }
                if effect.attack_speed != 0
                {
                    effect.update_atk_speed(entity);
                }
                if effect.item_speed != 0
                {
                    effect.update_item_speed(entity);
                }
                if effect.base_damage != 0
                {
                    effect.update_base_damage(entity);
                }
                if effect.money != 0
                {
                    effect.update_money(entity);
                }
                entity.update_health_bar();
                true
            }
            else { false } // Effect has been removed; don't reschedule.
        }){
            Some(response) => response,
            None => false
        }
    })
}

/**
 * This will update the original effect to ensure
 * that it can be removed correctly.
 */
fn updatable_effect(potion_ref: &'static str, accessor: EntityAccessor) -> Box<'static + Fn() -> bool>
{
    Box::new(move ||
    {
        match access::entity(accessor, |entity |
        {
            if let Some(ref player) = entity.as_player()
            {
                player.update_effect(potion_ref, | effect |
                {
                    if effect.max_health != 0
                    {
                        effect.mut_update_max_health(entity);
                    }
                    if effect.health != 0
                    {
                        effect.mut_update_health(entity);
                    }
                    if effect.attack_speed != 0
                    {
                        effect.mut_update_atk_speed(entity);
                    }
                    if effect.item_speed != 0
                    {
                        effect.mut_update_item_speed(entity);
                    }
                    if effect.base_damage != 0
                    {
                        effect.mut_update_base_damage(entity);
                    }
                    if effect.money != 0
                    {
                        effect.mut_update_money(entity);
                    }
                    entity.update_health_bar();
                })
            }
            else { false }
        }) {
            Some(response) => response,
            None => false
        }
    })
}