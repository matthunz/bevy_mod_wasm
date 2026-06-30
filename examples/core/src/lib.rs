use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "host", derive(bevy::ecs::resource::Resource))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Score {
    pub value: u32,
}
