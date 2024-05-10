use crate::tipos_de_dato::comandos::cat_file;
use crate::tipos_de_dato::logger::Logger;
use crate::utils::{self, io};
use crate::utils::{compresion, objects};
use flate2::{Decompress, FlushDecompress};
use sha1::{Digest, Sha1};
use std::path::PathBuf;
use std::str;
use std::sync::Arc;

const COMMIT: u8 = 1;
const TREE: u8 = 2;
const BLOB: u8 = 3;
const TAG: u8 = 4;
const OFS_DELTA: u8 = 6;
const REF_DELTA: u8 = 7;

pub struct Packfile;

impl Packfile {
    // Devuelve el largo del objeto descomprimido junto a su tipo
    fn obtener_largo_y_tipo_de_directorio(
        objeto: String,
        dir: &str,
    ) -> Result<(u32, String), String> {
        let log = Arc::new(Logger::new(PathBuf::from("log.txt"))?);
        let tamanio_objeto_str =
            cat_file::CatFile::from(&mut vec!["-s".to_string(), objeto.clone()], log.clone())?
                .ejecutar_de(dir)?;

        let tamanio_objeto = tamanio_objeto_str.trim().parse::<u32>().unwrap_or(0);

        let tipo_objeto = cat_file::obtener_tipo_objeto_de(&objeto, dir)?;

        Ok((tamanio_objeto, tipo_objeto))
    }

    // Funcion que dado el hash de un objeto, lo aniade al packfile
    fn aniadir_objeto(
        objetos_packfile: &mut Vec<u8>,
        objeto: String,
        dir: &str,
    ) -> Result<(), String> {
        let (tamanio_objeto, tipo_objeto) =
            Self::obtener_largo_y_tipo_de_directorio(objeto.clone(), dir)?;

        // codifica el tamanio del archivo descomprimido y su tipo en un tipo variable de longitud
        let nbyte = match tipo_objeto.as_str() {
            "commit" => Self::codificar_bytes(COMMIT, tamanio_objeto), //1
            "tree" => Self::codificar_bytes(TREE, tamanio_objeto),     // 2
            "blob" => Self::codificar_bytes(BLOB, tamanio_objeto),     // 3
            "tag" => Self::codificar_bytes(TAG, tamanio_objeto),       // 4
            "ofs_delta" => Self::codificar_bytes(OFS_DELTA, tamanio_objeto), // 6
            "ref_delta" => Self::codificar_bytes(REF_DELTA, tamanio_objeto), // 7
            _ => {
                return Err("Tipo de objeto invalido".to_string());
            }
        };
        // El objeto manda comprimido pero sin header
        let obj = utils::compresion::obtener_contenido_comprimido_sin_header_de(&objeto, dir)?;
        objetos_packfile.extend(nbyte);
        objetos_packfile.extend(obj);

        Ok(())
    }

    // Funcion que dado un directorio extrae todos sus objetos y los aniade al packfile
    fn obtener_objetos_del_dir(dir: &str) -> Result<(Vec<u8>, u32), String> {
        // esto porque es un clone, deberia pasarle los objetos que quiero
        let mut objetos_packfile: Vec<u8> = Vec::new();
        let objetos = objects::obtener_objetos_del_dir(&PathBuf::from(dir))?;
        let cant_objetos = objetos.len() as u32;
        // ---
        for objeto in objetos {
            Self::aniadir_objeto(&mut objetos_packfile, objeto.clone(), dir)?;
        }
        Ok((objetos_packfile, cant_objetos))
    }

    /// Dado un directorio, arma el packfile en base a los objetos del mismo y lo devuelve
    pub fn obtener_pack_entero(dir: &str) -> Result<Vec<u8>, String> {
        println!("Despachando packfile");
        let (objetos_packfile, cant_objetos) = Self::obtener_objetos_del_dir(dir)?;

        Ok(Self::armar_packfile(objetos_packfile, cant_objetos))
    }

    /// Dado un directorio y un vector de objetos, arma el packfile en base a los objetos del mismo y lo devuelve
    pub fn obtener_pack_con_archivos(objetos: Vec<String>, dir: &str) -> Result<Vec<u8>, String> {
        let mut objetos_packfile: Vec<u8> = Vec::new();

        let cant_objetos = objetos.len() as u32;
        for objeto in objetos {
            Self::aniadir_objeto(&mut objetos_packfile, objeto, dir)?;
        }
        Ok(Self::armar_packfile(objetos_packfile, cant_objetos))
    }

    // Dado un vector de bytes y el offset absoluto de un objeto junto a su tamanio descomprimido, devuelve el objeto descomprimido
    fn descomprimir_objeto(
        bytes: &[u8],
        offset: &mut usize,
        tamanio_objeto_descomprimido: u32,
    ) -> Result<Vec<u8>, String> {
        let mut objeto_descomprimido = vec![0; tamanio_objeto_descomprimido as usize];

        let mut descompresor = Decompress::new(true);

        descompresor
            .decompress(
                &bytes[*offset..],
                &mut objeto_descomprimido,
                FlushDecompress::None,
            )
            .map_err(|e| e.to_string())?;

        *offset += descompresor.total_in() as usize;
        Ok(objeto_descomprimido)
    }

    // Dado un vector de bytes conformado por objetos y la cantidad de objetos, devuelve el packfile armado
    fn armar_packfile(objetos: Vec<u8>, cant_objetos: u32) -> Vec<u8> {
        let mut packfile: Vec<u8> = Vec::new();
        // header
        packfile.extend("PACK".as_bytes());
        packfile.extend(2u32.to_be_bytes());
        packfile.extend(&cant_objetos.to_be_bytes());
        // objetos
        packfile.extend(&objetos);
        // checksum
        let mut hasher = Sha1::new();
        hasher.update(&packfile);
        let hash = hasher.finalize();
        packfile.extend(&hash);

        packfile
    }

    // Codifica el tipo de un objeto y el largo del mismo, codifica en bytes de tipo de longitud variable (conocidos como varint)
    fn codificar_bytes(tipo: u8, largo_objeto: u32) -> Vec<u8> {
        let mut resultado = Vec::new();
        let mut valor = largo_objeto;
        // si lo el tamanio del numero es mayor a 4 bits, entonces tengo que poner el bit mas significativo en 1
        let primer_byte: u8 = if valor >> 4 != 0 {
            ((tipo & 0x07) << 4) | 0x80 | (largo_objeto & 0x0F) as u8
        } else {
            ((tipo & 0x07) << 4) | (largo_objeto & 0x0F) as u8
        };

        resultado.push(primer_byte);
        valor >>= 4;
        loop {
            if valor == 0 {
                break;
            }
            let mut byte = (valor & 0x7F) as u8;
            valor >>= 7;

            if valor > 0 {
                byte |= 0x80;
            }
            resultado.push(byte);
        }

        resultado
    }

    // Dado un vector de bytes y un offset absoluto del mismo, decodifica el tipo y el largo de un objeto
    fn decodificar_bytes(bytes: &[u8], offset: &mut usize) -> (u8, u32) {
        let mut numero_decodificado: u32;
        let mut corrimiento: u32 = 0;
        let mut continua = false;

        // decodifico el primer byte que es distinto
        let tipo = &bytes[*offset] >> 4 & 0x07; // deduzco el tipo
        numero_decodificado = (bytes[*offset] & 0x0f) as u32; // obtengo los primeros 4 bits

        if bytes[*offset] & 0x80 != 0 {
            continua = true;
        }
        *offset += 1;
        corrimiento += 4;
        loop {
            if !continua {
                break;
            }
            if bytes[*offset] & 0x80 == 0 {
                continua = false;
            }
            numero_decodificado |= ((&bytes[*offset] & 0x7f) as u32) << corrimiento;
            corrimiento += 7;
            *offset += 1;
        }
        (tipo, numero_decodificado)
    }

    // Dado el tipo, el tamanio y el contenido descomprimido de un objeto, devuelve el objeto con su header
    fn obtener_objeto_con_header(
        tipo: u8,
        tamanio: u32,
        contenido_descomprimido: &mut Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let mut header: Vec<u8> = match tipo {
            1 => format!("{} {}\0", "commit", tamanio).as_bytes().to_vec(),
            2 => format!("{} {}\0", "tree", tamanio).as_bytes().to_vec(),
            3 => format!("{} {}\0", "blob", tamanio).as_bytes().to_vec(),
            _ => {
                return Err("Tipo de objeto invalido".to_string());
            }
        };
        header.append(contenido_descomprimido);
        Ok(header)
    }

    // Verifica el checksum de un packfile
    pub fn verificar_checksum(packfile: &[u8]) -> bool {
        let expected_hash = &packfile[packfile.len() - 20..];

        let mut hasher = Sha1::new();
        hasher.update(&packfile[..packfile.len() - 20]);
        let actual_hash = hasher.finalize();

        expected_hash == actual_hash.as_slice()
    }

    // Lee el header del packfile y devuelve la firma, la version y el largo
    fn leer_header_packfile(packfile: &[u8]) -> Result<(&[u8], &[u8], u32), String> {
        let firma = &packfile[0..4];
        let version = &packfile[4..8];
        let largo = &packfile[8..12];
        let largo = u32::from_be_bytes([largo[0], largo[1], largo[2], largo[3]]);
        Ok((firma, version, largo))
    }

    /// Dado un packfile (en forma de Vec<u8>) y una ubicacion, decodifica los objetos dentro del mismo, y los escribe en la ubicacion dada
    pub fn leer_packfile_y_escribir(bytes: &[u8], ubicacion: String) -> Result<(), String> {
        let checksum = Self::verificar_checksum(bytes);
        match checksum {
            true => println!("Checksum correcto"),
            false => println!("Checksum incorrecto"),
        }
        let (_firma, _version, largo) = Self::leer_header_packfile(bytes)?;
        let mut offset = 12;

        let mut contador: u32 = 0;

        while contador < largo {
            let (tipo, mut objeto) = Self::leer_objeto_del_packfile(bytes, &mut offset)?;

            let objeto = Self::obtener_objeto_con_header(tipo, objeto.len() as u32, &mut objeto)?;

            let mut hasher = Sha1::new();
            hasher.update(objeto.clone());
            let _hash = hasher.finalize();
            let hash = format!("{:x}", _hash);

            let ruta = format!("{}{}/{}", &ubicacion, &hash[..2], &hash[2..]);
            io::escribir_bytes(ruta, compresion::comprimir_contenido_u8(&objeto)?)?;
            contador += 1;
        }
        Ok(())
    }

    // Funcion para leer un varint de un vector de bytes en formato big endian, la forma en la que se procesa tiene que ver con su codificacion
    // ya que es de un objeto delta
    fn leer_vli_be(bytes: &[u8], actual_offset: &mut usize, offset: bool) -> usize {
        let mut val: usize = 0;
        loop {
            let byt = &bytes[*actual_offset];
            *actual_offset += 1;
            val = (val << 7) | (byt & 0x7f) as usize;
            if byt & 0x80 == 0 {
                break;
            }
            if offset {
                val += 1
            }
        }
        val
    }

    // Funcion para leer un varint de un vector de bytes en formato little endian
    fn leer_objeto_del_packfile(bytes: &[u8], offset: &mut usize) -> Result<(u8, Vec<u8>), String> {
        let offset_pre_varint = *offset;
        let (tipo, tamanio) = Self::decodificar_bytes(bytes, offset);
        if tipo == 6 {
            Self::leer_ofs_delta_obj(bytes, tamanio, offset, offset_pre_varint)
        } else {
            let objeto_descomprimido = Self::descomprimir_objeto(bytes, offset, tamanio)?;
            Ok((tipo, objeto_descomprimido))
        }
    }

    // Funcion para decodificar ofs delta. Devuelve el tipo y el objeto reconstruido y descomprimido
    fn leer_ofs_delta_obj(
        bytes: &[u8],
        obj_size: u32,
        actual_offset: &mut usize,
        offset_pre_varint: usize,
    ) -> Result<(u8, Vec<u8>), String> {
        let offset = Self::leer_vli_be(bytes, actual_offset, true);

        let base_obj_offset = offset_pre_varint - offset;

        let (base_obj_type, mut base_obj_data) =
            Self::leer_objeto_del_packfile(bytes, &mut { base_obj_offset })?;

        Self::crear_delta_obj(
            bytes,
            actual_offset,
            base_obj_type,
            &mut base_obj_data,
            obj_size,
        )
    }

    // Funcion para procesar las instrucciones de reconstruccion de un objeto delta. Devuelve el tipo y el objeto reconstruido y descomprimido
    fn crear_delta_obj(
        bytes: &[u8],
        actual_offset: &mut usize,
        tipo_de_objeto_base: u8,
        data_objeto_base: &mut [u8],
        obj_size: u32,
    ) -> Result<(u8, Vec<u8>), String> {
        let objeto_descomprimido = Self::descomprimir_objeto(bytes, actual_offset, obj_size)?;

        let mut data_descomprimida_offset: usize = 0;
        let _tamanio_objeto_base =
            Self::leer_varint_le(&objeto_descomprimido, &mut data_descomprimida_offset);
        let _tamanio_objeto_reconstruido =
            Self::leer_varint_le(&objeto_descomprimido, &mut data_descomprimida_offset);

        let mut obj_data: Vec<u8> = Vec::new();

        while data_descomprimida_offset < objeto_descomprimido.len() {
            let byt = &objeto_descomprimido[data_descomprimida_offset];
            data_descomprimida_offset += 1;
            if *byt == 0x00 {
                continue;
            }
            if (byt & 0x80) != 0 {
                let mut vals: Vec<u8> = Vec::new();
                for i in 0..7 {
                    let mascara = 1 << i;
                    if (byt & mascara) != 0 {
                        vals.push(objeto_descomprimido[data_descomprimida_offset]);
                        data_descomprimida_offset += 1;
                    } else {
                        vals.push(0);
                    }
                }
                let inicio = u32::from_le_bytes([vals[0], vals[1], vals[2], vals[3]]) as usize;
                let mut nbytes: usize = u16::from_le_bytes([vals[4], vals[5]]) as usize;
                if nbytes == 0 {
                    nbytes = 0x10000
                }

                obj_data.extend(&data_objeto_base[inicio..inicio + nbytes]);
            } else {
                let nbytes = byt & 0x7f;
                obj_data.extend(
                    &objeto_descomprimido
                        [data_descomprimida_offset..data_descomprimida_offset + nbytes as usize],
                );
                data_descomprimida_offset += nbytes as usize;
            }
        }
        Ok((tipo_de_objeto_base, obj_data))
    }

    // Funcion que dado un vector de bytes y un offset absoluto del mismo, decodifica un variable length integer en formato little endian
    fn leer_varint_le(input: &[u8], offset: &mut usize) -> u32 {
        let mut result = 0u32;
        let mut shift = 0;

        loop {
            let byte = input[*offset];
            result |= ((byte & 0x7F) as u32) << shift;
            shift += 7;
            *offset += 1;

            if byte & 0x80 == 0 {
                break;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn leer_blob_de_packfile(packfile: &[u8], offset: &mut usize) -> (Vec<u8>, u8, u32) {
        let (tipo, tamanio) = Packfile::decodificar_bytes(packfile, offset);
        let mut objeto_descomprimido =
            Packfile::descomprimir_objeto(packfile, offset, tamanio).unwrap();
        let objeto = Packfile::obtener_objeto_con_header(
            tipo,
            objeto_descomprimido.len() as u32,
            &mut objeto_descomprimido,
        )
        .unwrap();
        (objeto, tipo, tamanio)
    }

    #[test]
    #[serial]
    fn test01_codificar_bytes_de_un_byte() {
        let tipo = 1;
        let largo = 10;
        let resultado = super::Packfile::codificar_bytes(tipo, largo);
        assert_eq!(resultado, vec![0x1a]);
    }

    #[test]
    #[serial]
    fn test02_codificar_bytes_de_mas_de_un_byte() {
        let tipo = 2;
        let largo = 100;
        let resultado = super::Packfile::codificar_bytes(tipo, largo);
        assert_eq!(resultado, vec![0xa4, 0x06]);
    }

    #[test]
    #[serial]
    fn test03_decodificar_un_byte() {
        let bytes = vec![0x1a];
        let mut offset = 0;
        let resultado = super::Packfile::decodificar_bytes(&bytes, &mut offset);
        assert_eq!(resultado, (1, 10));
    }

    #[test]
    #[serial]
    fn test04_decodificar_mas_de_un_byte() {
        let bytes = vec![0xa4, 0x06];
        let mut offset = 0;
        let resultado = super::Packfile::decodificar_bytes(&bytes, &mut offset);
        assert_eq!(resultado, (2, 100));
    }

    #[test]
    #[serial]
    fn test05_header_se_escribe_bien() {
        let packfile = Packfile::obtener_pack_con_archivos(Vec::new(), "").unwrap();
        let header = Packfile::leer_header_packfile(&packfile).unwrap();
        assert_eq!(header.0, "PACK".as_bytes());
        assert_eq!(header.1, 2u32.to_be_bytes());
        assert_eq!(header.2, 0);
    }

    #[test]
    #[serial]
    fn test06_obtener_pack_entero() {
        let dir = env!("CARGO_MANIFEST_DIR").to_string() + "/packfile_test_dir/"; // replace with a valid directory path
        let result = Packfile::obtener_pack_entero(&dir);

        assert!(result.is_ok());
        let packfile = result.unwrap();
        let mut offset: usize = 12;
        // 1er objeto

        let objeto = leer_blob_de_packfile(&packfile, &mut offset);
        assert_eq!(objeto.1, 3); // es de tipo 3
        let obj_leido = utils::compresion::descomprimir_contenido_u8(
            &io::leer_bytes(
                env!("CARGO_MANIFEST_DIR").to_string()
                    + "/packfile_test_dir/51/22b1de1b7a07e36b01cd62bd622a0715f92478",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(obj_leido, objeto.0);
        // 2do objeto

        let objeto = leer_blob_de_packfile(&packfile, &mut offset);
        assert_eq!(objeto.1, 3); // es de tipo 3
        let obj_leido = utils::compresion::descomprimir_contenido_u8(
            &io::leer_bytes(
                env!("CARGO_MANIFEST_DIR").to_string()
                    + "/packfile_test_dir/87/7e9f62c1031de82130f279f009469cc9e09ab0",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(obj_leido, objeto.0);
        assert!(Packfile::verificar_checksum(packfile.as_slice()));
    }

    #[test]
    #[serial]
    fn test07_obtener_pack_con_archivos() {
        let directorio = env!("CARGO_MANIFEST_DIR").to_string() + "/packfile_test_dir/";
        let dir_objeto = "51/22b1de1b7a07e36b01cd62bd622a0715f92478";
        let hash_objeto = "5122b1de1b7a07e36b01cd62bd622a0715f92478";
        let objetos = vec![hash_objeto.to_string()];
        let result = Packfile::obtener_pack_con_archivos(objetos, &directorio);
        assert!(result.is_ok());
        let packfile = result.unwrap();
        let mut offset = 12;
        let objeto = leer_blob_de_packfile(&packfile, &mut offset);
        let obj_leido = utils::compresion::descomprimir_contenido_u8(
            &io::leer_bytes(directorio + dir_objeto).unwrap(),
        )
        .unwrap();
        assert_eq!(obj_leido, objeto.0);
        assert_eq!(offset, packfile.len() - 20); // para asertar que solo queda el checksum
        assert!(Packfile::verificar_checksum(packfile.as_slice()));
    }
    #[test]
    #[serial]
    fn test08_leer_objeto_packfile() {
        let directorio = env!("CARGO_MANIFEST_DIR").to_string() + "/packfile_test_dir/";
        let dir_objeto = "51/22b1de1b7a07e36b01cd62bd622a0715f92478";
        let hash_objeto = "5122b1de1b7a07e36b01cd62bd622a0715f92478";
        let objetos = vec![hash_objeto.to_string()];
        let result = Packfile::obtener_pack_con_archivos(objetos, &directorio);
        assert!(result.is_ok());

        let packfile = result.unwrap();
        let mut offset = 12;
        let mut objeto = Packfile::leer_objeto_del_packfile(&packfile, &mut offset).unwrap();
        let objeto_con_header =
            Packfile::obtener_objeto_con_header(objeto.0, objeto.1.len() as u32, &mut objeto.1)
                .unwrap();
        let obj_leido = utils::compresion::descomprimir_contenido_u8(
            &io::leer_bytes(directorio + dir_objeto).unwrap(),
        )
        .unwrap();
        assert_eq!(obj_leido, objeto_con_header);
        assert_eq!(offset, packfile.len() - 20); // para asertar que solo queda el checksum
        assert!(Packfile::verificar_checksum(packfile.as_slice()));
    }

    #[test]
    fn test09_delta_ref() {
        let packfile =
            io::leer_bytes(env!("CARGO_MANIFEST_DIR").to_string() + "/packfile_test").unwrap();
        let mut offset = 12;
        let (_firma, _version, largo) = Packfile::leer_header_packfile(&packfile).unwrap();
        let mut contador = 0;
        while contador < largo {
            let objeto = Packfile::leer_objeto_del_packfile(&packfile, &mut offset);
            contador += 1;
            assert!(objeto.is_ok());
        }
    }
}
