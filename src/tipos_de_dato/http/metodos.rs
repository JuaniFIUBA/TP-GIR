use super::error::ErrorHttp;

#[derive(Debug, PartialEq)]
pub enum MetodoHttp {
    Get,
    Post,
    Put,
    Patch,
}

impl MetodoHttp {
    pub fn from_string(metodo: &str) -> Result<MetodoHttp, ErrorHttp> {
        match metodo {
            "GET" => Ok(MetodoHttp::Get),
            "POST" => Ok(MetodoHttp::Post),
            "PUT" => Ok(MetodoHttp::Put),
            "PATCH" => Ok(MetodoHttp::Patch),
            _ => Err(ErrorHttp::Forbidden(
                "El acceso ha sido denegado".to_string(),
            )),
        }
    }
}
