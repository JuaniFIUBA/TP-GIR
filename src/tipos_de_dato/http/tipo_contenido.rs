use std::collections::HashMap;

use super::error::ErrorHttp;

pub enum TipoContenido {
    Json,
    Xml,
    UrlEncoded,
}

impl TipoContenido {
    pub fn from_string(string: &str) -> Result<Self, String> {
        match string {
            "application/json" => Ok(Self::Json),
            "application/xml" => Ok(Self::Xml),
            "application/x-www-form-urlencoded" => Ok(Self::UrlEncoded),
            _ => Err(format!("Tipo de contenido {} no soportado", string)),
        }
    }

    pub fn parsear_contenido(
        &self,
        contenido: &[u8],
    ) -> Result<HashMap<String, String>, ErrorHttp> {
        let resultado = match self {
            Self::Json => serde_json::from_slice(contenido).ok(),
            Self::UrlEncoded => serde_urlencoded::from_bytes(contenido).ok(),
            Self::Xml => serde_xml_rs::from_reader(contenido).ok(),
        };

        resultado.ok_or(ErrorHttp::BadRequest(
            "No se ha podido parsear el contenido de la solicitud".to_string(),
        ))
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_parsear_contenido_json() {
        let contenido = r#"{
            "key": "value",
            "otra_key": "otro_value"
        }"#;

        let tipo_contenido = TipoContenido::from_string("application/json").unwrap();
        let json = tipo_contenido
            .parsear_contenido(contenido.as_bytes())
            .unwrap();

        let mut json_esperado = HashMap::new();
        json_esperado.insert("key".to_string(), "value".to_string());
        json_esperado.insert("otra_key".to_string(), "otro_value".to_string());

        assert_eq!(json, json_esperado);
    }

    #[test]
    fn test_parsear_contenido_xml() {
        let contenido = r#"<xml>
            <key>value</key>
            <otra_key>otro_value</otra_key>
        </xml>"#;

        let tipo_contenido = TipoContenido::from_string("application/xml").unwrap();
        let xml = tipo_contenido
            .parsear_contenido(contenido.as_bytes())
            .unwrap();

        let mut xml_esperado = HashMap::new();

        xml_esperado.insert("key".to_string(), "value".to_string());
        xml_esperado.insert("otra_key".to_string(), "otro_value".to_string());

        assert_eq!(xml, xml_esperado);
    }

    #[test]
    fn test_parsear_contenido_urlencoded() {
        let contenido = "key=value&otra_key=otro_value";

        let tipo_contenido =
            TipoContenido::from_string("application/x-www-form-urlencoded").unwrap();
        let urlencoded = tipo_contenido
            .parsear_contenido(contenido.as_bytes())
            .unwrap();

        let mut urlencoded_esperado = HashMap::new();

        urlencoded_esperado.insert("key".to_string(), "value".to_string());
        urlencoded_esperado.insert("otra_key".to_string(), "otro_value".to_string());

        assert_eq!(urlencoded, urlencoded_esperado);
    }
}
