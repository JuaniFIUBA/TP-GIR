use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    io::{BufRead, BufReader, Read, Write},
    sync::Arc,
};

use super::{error::ErrorHttp, metodos::MetodoHttp, tipo_contenido::TipoContenido};
use crate::tipos_de_dato::logger::Logger;
pub struct Request {
    pub metodo: MetodoHttp,
    pub ruta: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<HashMap<String, String>>,
    pub logger: Arc<Logger>,
}

impl Request {
    fn parsear_header_largo(option_largo: Option<&String>) -> Result<usize, ErrorHttp> {
        match option_largo {
            Some(largo_raw) => match largo_raw.parse::<usize>() {
                Ok(largo) => Ok(largo),
                Err(e) => Err(ErrorHttp::BadRequest(e.to_string())),
            },
            None => Err(ErrorHttp::BadRequest(
                "No se encontro el header Content-Length".to_string(),
            )),
        }
    }

    fn parsear_header_tipo(option_tipo: Option<&String>) -> Result<TipoContenido, ErrorHttp> {
        match option_tipo {
            Some(tipo_raw) => {
                let tipo = TipoContenido::from_string(tipo_raw)
                    .map_err(|e| ErrorHttp::BadRequest(e.to_string()))?;
                Ok(tipo)
            }
            None => Err(ErrorHttp::BadRequest(
                "No se encontro el header Content-Type".to_string(),
            )),
        }
    }

    fn obtener_headers_contenido(
        headers: &HashMap<String, String>,
    ) -> Result<Option<(usize, TipoContenido)>, ErrorHttp> {
        let option_largo = headers.get("Content-Length");
        let option_tipo = headers.get("Content-Type");
        if option_largo.is_none() && option_tipo.is_none() {
            return Ok(None);
        }

        let largo = Self::parsear_header_largo(option_largo)?;
        if largo == 0 {
            return Ok(None);
        }

        let tipo = Self::parsear_header_tipo(option_tipo)?;

        Ok(Some((largo, tipo)))
    }

    pub fn from<T>(reader: &mut BufReader<&mut T>, logger: Arc<Logger>) -> Result<Self, ErrorHttp>
    where
        T: Read + Write,
    {
        let (metodo, ruta, version) = Self::obtener_primera_linea(reader)?;

        let metodo = MetodoHttp::from_string(&metodo)?;

        let headers = Self::obtener_headers(reader)?;
        let body = Self::obtener_body(reader, &headers)?;

        Ok(Self {
            metodo,
            ruta,
            version,
            headers,
            body,
            logger,
        })
    }

    fn obtener_headers<T>(
        reader: &mut BufReader<&mut T>,
    ) -> Result<HashMap<String, String>, ErrorHttp>
    where
        T: Read + Write,
    {
        let mut headers = HashMap::new();

        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            let splitted = line.splitn(2, ':').collect::<Vec<&str>>();
            if splitted.len() != 2 {
                return Err(ErrorHttp::BadRequest("Error parseando headers".to_string()));
            }

            let key = splitted[0].trim().to_string();
            let value = splitted[1].trim().to_string();

            headers.insert(key, value);
        }

        Ok(headers)
    }

    fn obtener_primera_linea<T>(
        reader: &mut BufReader<&mut T>,
    ) -> Result<(String, String, String), ErrorHttp>
    where
        T: Read + Write,
    {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| {
            ErrorHttp::InternalServerError(format!(
                "No se pudo leer las lineas envias al server {e}"
            ))
        })?;
        let splitted = line.split_whitespace().collect::<Vec<&str>>();
        if splitted.len() != 3 {
            return Err(ErrorHttp::BadRequest(
                "Error parseando primera linea".to_string(),
            ));
        }

        let metodo = splitted[0].to_string();
        let ruta = splitted[1].to_string();
        let version = splitted[2].to_string();

        Ok((metodo, ruta, version))
    }

    fn obtener_body<T>(
        reader: &mut BufReader<&mut T>,
        headers: &HashMap<String, String>,
    ) -> Result<Option<HashMap<String, String>>, ErrorHttp>
    where
        T: Read + Write,
    {
        let headers = Self::obtener_headers_contenido(headers)?;

        let (largo, tipo) = match headers {
            Some((largo, tipo)) => (largo, tipo),
            None => return Ok(None),
        };

        let mut body_buf = vec![0; largo];
        let leidos = reader
            .read(&mut body_buf)
            .map_err(|e| ErrorHttp::InternalServerError(e.to_string()))?;

        if leidos != largo {
            return Err(ErrorHttp::BadRequest(
                "No se pudo leer el body completo".to_string(),
            ));
        }

        let body = tipo.parsear_contenido(&body_buf)?;

        Ok(Some(body))
    }
}

impl Debug for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRequest")
            .field("metodo", &self.metodo)
            .field("ruta", &self.ruta)
            .field("version", &self.version)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::utils::testing::MockTcpStream;

    use super::*;

    #[test]
    fn test01_from() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/request_test01")).unwrap());

        let mut mock_tcp = MockTcpStream {
            lectura_data: b"POST / HTTP/1.1\nHost: localhost:8000\nAuthorization: Bearer token\nContent-Length: 116\nContent-Type: application/json\n\r\n{\"title\":\"Amazing new feature\",\"body\":\"Please pull these awesome changes in!\",\"head\":\"octocat:rama\",\"base\":\"master\"}".to_vec(),
            escritura_data: vec![],
        };

        let mut reader = BufReader::new(&mut mock_tcp);

        let request = Request::from(&mut reader, logger).unwrap();

        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "localhost:8000".to_string());
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        headers.insert("Content-Length".to_string(), "116".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let mut body = HashMap::new();
        body.insert("title".to_string(), "Amazing new feature".to_string());
        body.insert(
            "body".to_string(),
            "Please pull these awesome changes in!".to_string(),
        );
        body.insert("head".to_string(), "octocat:rama".to_string());
        body.insert("base".to_string(), "master".to_string());

        assert_eq!(request.metodo, MetodoHttp::Post);
        assert_eq!(request.ruta, "/".to_string());
        assert_eq!(request.version, "HTTP/1.1".to_string());
        assert_eq!(request.headers, headers);
        assert_eq!(request.body.unwrap(), body);
    }

    #[test]
    fn test02_from_sin_headers_content_no_lee_body() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/request_test02")).unwrap());

        let mut mock_tcp = MockTcpStream {
            lectura_data: b"POST / HTTP/1.1\nHost: localhost:8000\nAuthorization: Bearer token\n\r\n{\"title\":\"Amazing new feature\",\"body\":\"Please pull these awesome changes in!\",\"head\":\"octocat:rama\",\"base\":\"master\"}".to_vec(),
            escritura_data: vec![],
        };

        let mut reader = BufReader::new(&mut mock_tcp);

        let request = Request::from(&mut reader, logger).unwrap();

        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "localhost:8000".to_string());
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        assert_eq!(request.metodo, MetodoHttp::Post);
        assert_eq!(request.ruta, "/".to_string());
        assert_eq!(request.version, "HTTP/1.1".to_string());
        assert_eq!(request.headers, headers);
        assert_eq!(request.body, None);
    }
    #[test]
    fn test03_from_con_content_length_0_no_lee_body() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/request_test03")).unwrap());

        let mut mock_tcp = MockTcpStream {
            lectura_data: b"POST / HTTP/1.1\nHost: localhost:8000\nAuthorization: Bearer token\nContent-Length: 0\n\r\n{\"title\":\"Amazing new feature\",\"body\":\"Please pull these awesome changes in!\",\"head\":\"octocat:rama\",\"base\":\"master\"}".to_vec(),
            escritura_data: vec![],
        };

        let mut reader = BufReader::new(&mut mock_tcp);

        let request = Request::from(&mut reader, logger).unwrap();

        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "localhost:8000".to_string());
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        headers.insert("Content-Length".to_string(), "0".to_string());

        assert_eq!(request.metodo, MetodoHttp::Post);
        assert_eq!(request.ruta, "/".to_string());
        assert_eq!(request.version, "HTTP/1.1".to_string());
        assert_eq!(request.headers, headers);
        assert_eq!(request.body, None);
    }

    #[test]
    #[should_panic]
    fn test03_from_sin_content_type_panickea() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/request_test03")).unwrap());

        let mut mock_tcp = MockTcpStream {
            lectura_data: b"POST / HTTP/1.1\nHost: localhost:8000\nAuthorization: Bearer token\nContent-Length: 116\n\r\n{\"title\":\"Amazing new feature\",\"body\":\"Please pull these awesome changes in!\",\"head\":\"octocat:rama\",\"base\":\"master\"}".to_vec(),
            escritura_data: vec![],
        };

        let mut reader = BufReader::new(&mut mock_tcp);

        Request::from(&mut reader, logger).unwrap();
    }

    #[test]
    #[should_panic]
    fn test04_from_sin_content_length_panickea() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/request_test04")).unwrap());

        let mut mock_tcp = MockTcpStream {
            lectura_data: b"POST / HTTP/1.1\nHost: localhost:8000\nAuthorization: Bearer token\nContent-Type: application/json\n\r\n{\"title\":\"Amazing new feature\",\"body\":\"Please pull these awesome changes in!\",\"head\":\"octocat:rama\",\"base\":\"master\"}".to_vec(),
            escritura_data: vec![],
        };

        let mut reader = BufReader::new(&mut mock_tcp);

        Request::from(&mut reader, logger).unwrap();
    }
    #[test]
    #[should_panic]
    fn test05_from_content_length_invalido() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/request_test05")).unwrap());

        let mut mock_tcp = MockTcpStream {
            lectura_data: b"POST / HTTP/1.1\nHost: localhost:8000\nAuthorization: Bearer token\nContent-Length: 116\nContent-Type: invalido\n\r\n{\"title\":\"Amazing new feature\",\"body\":\"Please pull these awesome changes in!\",\"head\":\"octocat:rama\",\"base\":\"master\"}".to_vec(),
            escritura_data: vec![],
        };

        let mut reader = BufReader::new(&mut mock_tcp);

        Request::from(&mut reader, logger).unwrap();
    }
}
