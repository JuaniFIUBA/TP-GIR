use crate::tipos_de_dato::comando::Ejecutar;

use super::info_dialog;

pub trait ComandoGui {
    fn ejecutar_gui(&mut self) -> Option<String>;
}

impl<T> ComandoGui for Result<T, String>
where
    T: Ejecutar,
{
    fn ejecutar_gui(&mut self) -> Option<String> {
        let comando_unwrappeado = match self {
            Ok(comando) => comando,
            Err(mensaje) => {
                info_dialog::mostrar_error(mensaje);
                return None;
            }
        };

        match comando_unwrappeado.ejecutar() {
            Ok(resultado) => Some(resultado),
            Err(err) => {
                info_dialog::mostrar_error(&err);
                None
            }
        }
    }
}
