use parking_lot::{Mutex, MutexGuard};
use std::sync::Arc;

mod biome;
mod block;
mod entity;
#[allow(clippy::all)]
mod inventory;
mod item;
mod particle;
mod simplified_block;

pub use biome::Biome;
pub use block::BlockKind;
pub use entity::EntityKind;
pub use inventory::{Area, InventoryBacking, Window};
pub use item::Item;
pub use particle::Particle;
pub use simplified_block::SimplifiedBlockKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub item: Item,
    pub count: u32,

    /// Damage to the item, if it's damageable.
    pub damage: Option<u32>,
}

impl ItemStack {
    /// Creates a new `ItemStack`.
    pub fn new(item: Item, count: u32) -> Self {
        Self {
            item,
            count,
            damage: None,
        }
    }

    /// Returns the item type for this `ItemStack`.
    pub fn item(&self) -> Item {
        self.item
    }

    /// Returns the number of items in this `ItemStack`.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Adds more items to this ItemStack. Returns the new count.
    pub fn add(&mut self, count: u32) -> u32 {
        self.count += count;
        self.count
    }

    /// Removes some items from this ItemStack. Returns whether there
    /// were enough items to be removed.
    pub fn remove(&mut self, count: u32) -> bool {
        self.count = match self.count.checked_sub(count) {
            Some(count) => count,
            None => return false,
        };
        true
    }

    /// Sets the item for this `ItemStack`.
    pub fn set_item(&mut self, item: Item) {
        self.item = item;
    }

    /// Sets the count for this `ItemStack`.
    pub fn set_count(&mut self, count: u32) {
        self.count = count;
    }

    /// Damages the item by the specified amount.
    /// If this returns `true`, then the item is broken.
    pub fn damage(&mut self, amount: u32) -> bool {
        match &mut self.damage {
            Some(damage) => {
                *damage += amount;
                if let Some(durability) = self.item.durability() {
                    *damage >= durability
                } else {
                    false
                }
            }
            None => false,
        }
    }
}

type Slot = Mutex<Option<ItemStack>>;

/// A handle to an inventory.
///
/// An inventory is composed of one or more _areas_, each
/// if which contains one or more item stacks stored in an array. Areas are defined
/// by the `Area` enum; examples include `Storage`, `Hotbar`, `Helmet`, `Offhand`,
/// and `CraftingInput`.
///
/// Note that an `Inventory` is a _handle_; it's backed by an `Arc`. As such, cloning
/// it is cheap and creates a new handle to the same inventory. Interior mutability
/// is used to make this safe.
#[derive(Debug, Clone)]
pub struct Inventory {
    backing: Arc<InventoryBacking<Slot>>,
}

impl Inventory {
    /// Returns whether two `Inventory` handles point to the same
    /// backing inventory.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.backing, &other.backing)
    }

    /// Gets the item at the given index within an area in this inventory.
    ///
    /// The returned value is a `MutexGuard` and can be mutated.
    ///
    /// # Note
    /// _Never_ keep two returned `MutexGuard`s for the same inventory alive
    /// at once. Deadlocks are not fun.
    pub fn item(&self, area: Area, slot: usize) -> Option<MutexGuard<Option<ItemStack>>> {
        let slice = self.backing.area_slice(area)?;
        slice.get(slot).map(Mutex::lock)
    }

    /// Creates a new handle to the same inventory.
    ///
    /// This operation is the same as calling `clone()`, but it's more explicit
    /// in its intent.
    pub fn new_handle(&self) -> Inventory {
        self.clone()
    }
}