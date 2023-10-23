use proto_ecs::data_group::DataGroupID;

pub type StageID = u8;

/// Number of stages supported by the engine
pub const STAGE_COUNT: usize = StageID::MAX as usize + 1;

/// Stage Map type
pub type StageMap<F> = [Option<F>; STAGE_COUNT];

#[derive(Debug, Clone, Copy)]
pub enum Dependency {
    DataGroup(DataGroupID),
    OptionalDG(DataGroupID),
}

impl Dependency {
    pub fn unwrap(self) -> DataGroupID {
        match self {
            Dependency::OptionalDG(d) => d,
            Dependency::DataGroup(d) => d,
        }
    }
}
