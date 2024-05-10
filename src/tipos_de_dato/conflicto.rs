use super::{lado_conflicto::LadoConflicto, tipo_diff::TipoDiff};

pub type Conflicto = Vec<(TipoDiff, LadoConflicto)>;
