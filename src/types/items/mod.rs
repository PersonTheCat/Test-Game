pub mod bows;
pub mod curses;
pub mod inventories;
pub mod potions;
//pub mod staves;
pub mod swords;
pub mod consumables;
pub mod shops;
pub mod display_info;
pub mod keys;
pub mod pass_books;
pub mod item_settings;

/**
 * To-do: move this data elsewhere.
 */

pub const INF_USES: u32 = 0x10000;

pub fn format_num_uses(num_uses: u32, max_uses: u32) -> String
{
    if max_uses == INF_USES
    {
        String::from("∞")
    }
    else { format!("{} / {}", num_uses, max_uses) }
}

pub fn format_damage(damage: u32, speed: u32) -> String
{
    format!("{}d / {:.1}s", damage, (speed as f32) / 1000.0)
}

pub fn format_damage_2(damage: u32, speed: i32) -> String
{
    format!("{}d / {:.1}s", damage, (speed as f32) / 1000.0)
}