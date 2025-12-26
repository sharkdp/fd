pub use self::size::SizeFilter;
pub use self::time::TimeFilter;

#[cfg(unix)]
pub use self::owner::OwnerFilter;

mod size;
mod time;

#[cfg(unix)]
mod owner;

pub trait Filter {
    type Item;

    fn matches(&self, item: &Self::Item) -> bool;
}
