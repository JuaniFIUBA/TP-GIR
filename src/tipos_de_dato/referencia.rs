use std::path::PathBuf;

use crate::utils;

pub enum Referencia {
    //1- Es la referencia a mandar 2- La referencia a ser enviada
    RamaMerge(String, String),
    //1- Es la referencia a mandar 2- La referencia a ser enviada
    Tag(String, String),
}

impl Referencia {
    pub fn from(referencia: String) -> Option<Referencia> {
        let (ref_local, ref_remota) = Self::divir_referencia(referencia);

        if utils::tags::existe_tag(&ref_local) {
            Some(Referencia::Tag(ref_local, ref_remota))
        } else if utils::ramas::existe_la_rama(&ref_local) {
            Some(Referencia::RamaMerge(ref_local, ref_remota))
        } else {
            None
        }
    }

    fn divir_referencia(referencia: String) -> (String, String) {
        match referencia.split_once(':') {
            Some((ref_local, ref_remota)) => (ref_local.to_string(), ref_remota.to_string()),
            None => (referencia.clone(), referencia),
        }
    }

    pub fn es_tag(&self) -> bool {
        matches!(self, Referencia::Tag(_, _))
    }

    pub fn dar_nombre_local(&self) -> String {
        match self {
            Referencia::Tag(ref_local, _) => ref_local.clone(),
            Referencia::RamaMerge(ref_local, _) => ref_local.clone(),
        }
    }

    pub fn dar_nombre_remoto(&self) -> String {
        match self {
            Referencia::Tag(_, ref_remota) => ref_remota.clone(),
            Referencia::RamaMerge(_, ref_remota) => ref_remota.clone(),
        }
    }

    pub fn dar_ref_local(&self) -> PathBuf {
        match self {
            Referencia::Tag(ref_local, _) => PathBuf::from(format!("refs/tags/{}", ref_local)),
            Referencia::RamaMerge(ref_local, _) => {
                PathBuf::from(format!("refs/heads/{}", ref_local))
            }
        }
    }

    pub fn dar_ref_remota(&self) -> PathBuf {
        match self {
            Referencia::Tag(_, ref_remota) => PathBuf::from(format!("refs/tags/{}", ref_remota)),
            Referencia::RamaMerge(_, ref_remota) => {
                PathBuf::from(format!("refs/heads/{}", ref_remota))
            }
        }
    }
}
