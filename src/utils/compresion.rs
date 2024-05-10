use crate::tipos_de_dato::objetos::tree::Tree;
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};

use super::io;

/// Dado un hash y una ruta, busca el archivo de ese hash en la ruta especificada
/// y devuelve el contenido del objeto descomprimido.
pub fn descomprimir_objeto(hash: &str, ruta: &str) -> Result<String, String> {
    let ruta_objeto = format!("{}{}/{}", ruta, &hash[..2], &hash[2..]);

    let contenido_leido = io::leer_bytes(ruta_objeto)?;
    let contenido_descomprimido = descomprimir_contenido_u8(&contenido_leido)?;
    let contenido_decodificado = decodificar_contenido(contenido_descomprimido)?;
    Ok(contenido_decodificado)
}

/// Descomprime el objeto indicado por el hash en la ruta .gir/objects
pub fn descomprimir_objeto_gir(hash: &str) -> Result<String, String> {
    descomprimir_objeto(hash, ".gir/objects/")
}

/// Convierte un vector de u8 a un string.
/// Si el vector no es valido, devuelve un error.
pub fn vec_a_string(vec: Vec<u8>) -> Result<String, String> {
    match String::from_utf8(vec) {
        Ok(string) => Ok(string),
        Err(_) => Err("No se pudo convertir el vec a string".to_string()),
    }
}

/// Devuelve el contenido decodificado de un objeto, sirve en especial para los trees ya que
/// estos tienen un formato donde el hash se almacena en binario
pub fn decodificar_contenido(contenido: Vec<u8>) -> Result<String, String> {
    let header_u8: &[u8] = contenido.split(|&x| x == 0).collect::<Vec<&[u8]>>()[0];

    let header = vec_a_string(header_u8.to_vec())?;
    let tipo_objeto = header.split_whitespace().collect::<Vec<&str>>()[0];

    match tipo_objeto {
        "blob" | "commit" => Ok(String::from_utf8_lossy(&contenido).to_string()),
        "tree" => decodificar_tree(&header, &contenido),
        _ => Err("Tipo de objeto invalido".to_string()),
    }
}

/// Separa el contenido que viene en un tree en lineas,
/// pasando de un [[hash][modo] [nombre], ...] a [[hash], [modo y nombre], ...]
fn separar_contenido_por_linea(contenido: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let mut spliteado_por_null: Vec<Vec<u8>> = Vec::new();
    let mut buffer: Vec<u8> = Vec::new();
    let mut i = 0;

    for char in contenido.iter() {
        if *char == 0 && (buffer.len() >= 20 || i < 2) {
            spliteado_por_null.push(buffer.clone());
            buffer.clear();
            i += 1;
        } else {
            buffer.push(*char);
        }
    }
    spliteado_por_null.push(buffer);

    let mut spliteado_por_null_separado_por_linea: Vec<Vec<u8>> = Vec::new();
    spliteado_por_null_separado_por_linea.push(spliteado_por_null[0].clone()); // tree
    spliteado_por_null_separado_por_linea.push(spliteado_por_null[1].clone()); // size

    let last_line = spliteado_por_null.pop(); // saco ultima que es hash

    spliteado_por_null.iter().skip(2).for_each(|x| {
        let (hash, modo_y_nombre) = x.split_at(20);
        spliteado_por_null_separado_por_linea.push(hash.to_vec());
        spliteado_por_null_separado_por_linea.push(modo_y_nombre.to_vec());
    });

    spliteado_por_null_separado_por_linea.push(
        last_line
            .ok_or("formato del objeto incorrecto".to_string())?
            .clone(),
    );
    Ok(spliteado_por_null_separado_por_linea)
}

/// Toma el vector [[hash], [modo y nombre], ...] y lo convierte nuevamente en un string con el formato
/// [header]\0[modo] [nombre]\0[hash]\0[modo] [nombre]\0[hash]\0...
fn reconstruir_contenido_separado(header: &str, contenido: Vec<Vec<u8>>) -> Result<String, String> {
    let mut contenido_decodificado = format!("{}\0", header);

    for i in (0..(contenido.len())).skip(1).step_by(2) {
        if i + 1 < contenido.len() {
            let modo_y_nombre = vec_a_string(contenido[i].clone())?;
            let hash = Tree::encode_hex(&contenido[i + 1]);

            let linea = format!("{modo_y_nombre}\0{hash}");
            contenido_decodificado.push_str(&linea);
        } else {
            return Err("Error al decodificar el contenido del tree".to_string());
        }
    }

    Ok(contenido_decodificado)
}

/// Decodifica el contenido u8 de un tree a String.
fn decodificar_tree(header: &str, contenido: &[u8]) -> Result<String, String> {
    let spliteado_por_null_separado_por_linea = separar_contenido_por_linea(contenido)?;

    reconstruir_contenido_separado(header, spliteado_por_null_separado_por_linea)
}

/// Comprime el contenido en String de un objeto.
/// Si el contenido no es valido, devuelve un error.
pub fn comprimir_contenido(contenido: &str) -> Result<Vec<u8>, String> {
    let mut compresor = ZlibEncoder::new(Vec::new(), Compression::default());
    if compresor.write_all(contenido.as_bytes()).is_err() {
        return Err("No se pudo comprimir el contenido".to_string());
    };
    match compresor.finish() {
        Ok(contenido_comprimido) => Ok(contenido_comprimido),
        Err(_) => Err("No se pudo comprimir el contenido".to_string()),
    }
}

/// Comprime el contenido en u8 de un objeto.
/// Si el contenido no es valido, devuelve un error.
pub fn comprimir_contenido_u8(contenido: &[u8]) -> Result<Vec<u8>, String> {
    let mut compresor = ZlibEncoder::new(Vec::new(), Compression::default());
    if compresor.write_all(contenido).is_err() {
        return Err("No se pudo comprimir el contenido".to_string());
    };
    match compresor.finish() {
        Ok(contenido_comprimido) => Ok(contenido_comprimido),
        Err(_) => Err("No se pudo comprimir el contenido".to_string()),
    }
}

/// Descomprime el contenido en u8 de un objeto.
pub fn descomprimir_contenido_u8(contenido: &[u8]) -> Result<Vec<u8>, String> {
    let mut descompresor = ZlibDecoder::new(contenido);
    let mut contenido_descomprimido = Vec::new();
    match descompresor.read_to_end(&mut contenido_descomprimido) {
        Ok(_) => {}
        Err(_) => Err("No se pudo descomprimir el contenido")?,
    };
    Ok(contenido_descomprimido)
}

/// Dado un hash y una ruta, busca el archivo de ese hash en la ruta especificada
/// y devuelve el contenido del objeto comprimido, sin tener en cuenta la linea del header del objeto.
pub fn obtener_contenido_comprimido_sin_header(hash: &str) -> Result<Vec<u8>, String> {
    let ruta_objeto = format!(".gir/objects/{}/{}", &hash[..2], &hash[2..]);
    let contenido_leido = io::leer_bytes(ruta_objeto)?;
    let cont_descomprimido = descomprimir_contenido_u8(&contenido_leido).unwrap();
    let vec: Vec<&[u8]> = cont_descomprimido.splitn(2, |&x| x == 0).collect();

    let contenido = vec[1];
    let contenido_comprimido = comprimir_contenido_u8(contenido)?;
    Ok(contenido_comprimido)
}

pub fn obtener_contenido_comprimido_sin_header_de(
    hash: &str,
    dir: &str,
) -> Result<Vec<u8>, String> {
    let ruta_objeto = format!("{}{}/{}", dir, &hash[..2], &hash[2..]);
    let contenido_leido = io::leer_bytes(ruta_objeto)?;
    let cont_descomprimido = descomprimir_contenido_u8(&contenido_leido).unwrap();
    let vec: Vec<&[u8]> = cont_descomprimido.splitn(2, |&x| x == 0).collect();

    let contenido = vec[1];
    let contenido_comprimido = comprimir_contenido_u8(contenido)?;
    Ok(contenido_comprimido)
}
