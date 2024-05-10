use std::{
    collections::HashMap,
    fmt::{Display, Write},
    fs,
    num::ParseIntError,
    path::{Path, PathBuf},
    sync::Arc,
};

use sha1::{Digest, Sha1};

use crate::{
    tipos_de_dato::{
        comando::Ejecutar,
        comandos::{cat_file, check_ignore::CheckIgnore, hash_object::HashObject, merge::Merge},
        logger::Logger,
        objeto::Objeto,
        tipo_diff::TipoDiff,
    },
    utils::path_buf::{esta_directorio_habilitado, obtener_nombre},
    utils::{
        compresion::{comprimir_contenido_u8, descomprimir_objeto},
        io,
    },
};

use super::blob::Blob;

#[derive(Clone, Debug)]

pub struct Tree {
    /// Directorio del arbol.
    pub directorio: PathBuf,
    /// Objetos que contiene el arbol.
    pub objetos: Vec<Objeto>,
    /// Logger para imprimir mensajes en el archivo log.
    pub logger: Arc<Logger>,
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.obtener_hash() == other.obtener_hash()
    }
}

impl Eq for Tree {}

impl Tree {
    /// Devuelve un vector con todos los objetos de tipo Blob que se encuentran en el arbol.
    pub fn obtener_objetos_hoja(&self) -> Vec<Objeto> {
        let mut objetos: Vec<Objeto> = Vec::new();
        for objeto in &self.objetos {
            match objeto {
                Objeto::Blob(blob) => objetos.push(Objeto::Blob(blob.clone())),
                Objeto::Tree(tree) => {
                    objetos.extend(tree.obtener_objetos_hoja());
                }
            }
        }
        objetos
    }

    /// Devuelve un vector con todos los objetos que se encuentran en el arbol.
    /// Si el arbol contiene un objeto de tipo Tree, tambien se lo incluye y se llama recursivamente a la funcion.
    pub fn obtener_objetos(&self) -> Vec<Objeto> {
        let mut objetos: Vec<Objeto> = Vec::new();
        for objeto in &self.objetos {
            match objeto {
                Objeto::Blob(blob) => objetos.push(Objeto::Blob(blob.clone())),
                Objeto::Tree(tree) => {
                    objetos.push(Objeto::Tree(tree.clone()));
                    objetos.extend(tree.obtener_objetos());
                }
            }
        }
        objetos
    }

    /// Devuelve un hashmap con todos los elementos que difieren.
    /// La key del hasmap es el nombre del archivo.
    /// El value es un vector de tuplas con el numero de linea y el tipo de diff.
    pub fn deep_changes(
        &self,
        arbol_a_comparar: &Tree,
    ) -> Result<HashMap<String, Vec<(usize, TipoDiff)>>, String> {
        let mut deep_diffs: HashMap<String, Vec<(usize, TipoDiff)>> = HashMap::new();

        for objeto in &self.objetos {
            for objeto_a_comparar in arbol_a_comparar.obtener_objetos() {
                if objeto.obtener_path() == objeto_a_comparar.obtener_path() {
                    if objeto.obtener_hash() == objeto_a_comparar.obtener_hash() {
                        break;
                    }
                    match objeto_a_comparar {
                        Objeto::Tree(ref tree_a_comparar) => {
                            if let Objeto::Tree(tree) = objeto {
                                let diff_hijos = tree.deep_changes(tree_a_comparar)?;
                                deep_diffs.extend(diff_hijos);
                            }
                        }
                        Objeto::Blob(blob_a_comparar) => {
                            if let Objeto::Blob(blob) = objeto {
                                let contenido_1 = cat_file::obtener_contenido_objeto(&blob.hash)?.1;
                                let contenido_2 =
                                    cat_file::obtener_contenido_objeto(&blob_a_comparar.hash)?.1;
                                let diff = Merge::obtener_diff(
                                    contenido_1.lines().collect(),
                                    contenido_2.lines().collect(),
                                );
                                deep_diffs
                                    .insert(blob.ubicacion.to_string_lossy().to_string(), diff);
                            }
                        }
                    }
                }
            }
        }

        Ok(deep_diffs)
    }

    /// Escribe en el directorio actual los archivos que se encuentran en el arbol
    pub fn escribir_en_directorio(&self) -> Result<(), String> {
        let objetos = self.obtener_objetos_hoja();
        for objeto in objetos {
            match objeto {
                Objeto::Blob(blob) => {
                    let objeto = descomprimir_objeto(&blob.hash, ".gir/objects/")?;
                    let contenido = objeto.split('\0').collect::<Vec<&str>>()[1];
                    io::escribir_bytes(blob.ubicacion, contenido).unwrap();
                }
                Objeto::Tree(_) => Err("Llego a un tree pero no deberia")?,
            };
        }
        Ok(())
    }

    /// Pasa un string de hexadecimal a un vector de u8.
    pub fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
        match (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<u8>, ParseIntError>>()
        {
            Ok(hex) => Ok(hex),
            Err(_) => Err(format!("Error al decodificar el hash {}", s)),
        }
    }

    /// Pasa un vector de u8 a un string de hexadecimal.
    pub fn encode_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            write!(&mut s, "{:02x}", b).unwrap();
        }
        s
    }

    /// Devuelve el hash completo de un objeto.
    pub fn obtener_hash(&self) -> Result<String, String> {
        let contenido = Self::obtener_contenido(&self.objetos)?;
        let mut sha1 = Sha1::new();

        let header = format!("tree {}\0", contenido.len());
        sha1.update([header.as_bytes(), &contenido].concat());

        let hash = sha1.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Devuelve el contenido de un objeto tree en u8 para poder ser comprimido.
    /// El contenido del objeto tree es el header del objeto y el entry de cada uno de sus hijos.
    /// Siguiendo los formatos de git.
    pub fn obtener_contenido(objetos: &[Objeto]) -> Result<Vec<u8>, String> {
        let objetos_ordenados = Self::ordenar_objetos_alfabeticamente(objetos);

        let mut contenido: Vec<u8> = Vec::new();

        for objeto in objetos_ordenados {
            let mut line = match objeto {
                Objeto::Blob(ref blob) => {
                    let hash = Self::decode_hex(&blob.hash)?;
                    [b"100644 ", blob.nombre.as_bytes(), b"\0", &hash].concat()
                }
                Objeto::Tree(tree) => {
                    let nombre = if tree.directorio == PathBuf::from(".") {
                        String::from(".")
                    } else {
                        obtener_nombre(&tree.directorio.clone())?
                    };
                    let hash = &Self::decode_hex(&tree.obtener_hash()?)?;
                    [b"40000 ", nombre.as_bytes(), b"\0", hash].concat()
                }
            };
            contenido.append(&mut line);
        }
        Ok(contenido)
    }

    /// Devuelve un objeto Tree a partir de un directorio y un vector de directorios que se quieren.
    pub fn from_directorio(
        directorio: PathBuf,
        hijos_especificados: Option<&Vec<PathBuf>>,
        logger: Arc<Logger>,
    ) -> Result<Tree, String> {
        let mut objetos: Vec<Objeto> = Vec::new();

        let entradas = match fs::read_dir(&directorio) {
            Ok(entradas) => entradas,
            Err(_) => Err(format!("Error al leer el directorio {directorio:#?}"))?,
        };

        for entrada in entradas {
            let entrada = entrada
                .map_err(|_| format!("Error al leer entrada el directorio {directorio:#?}"))?;
            let path = entrada.path();

            if CheckIgnore::es_directorio_a_ignorar(&path, logger.clone())? {
                continue;
            }

            if let Some(hijos_especificados) = &hijos_especificados {
                if !esta_directorio_habilitado(&path, hijos_especificados) {
                    continue;
                }
            }

            let objeto = match fs::metadata(&path) {
                Ok(_) => Objeto::from_directorio(path, hijos_especificados, logger.clone())?,
                Err(_) => Err("Error al leer el archivo".to_string())?,
            };
            objetos.push(objeto);
        }

        Ok(Tree {
            directorio,
            objetos,
            logger,
        })
    }

    /// Devuelve un vector con todos los paths de los hijos del arbol.
    pub fn obtener_paths_hijos(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        for objeto in &self.objetos {
            match objeto {
                Objeto::Blob(blob) => paths.push(blob.ubicacion.clone()),
                Objeto::Tree(tree) => {
                    paths.push(tree.directorio.clone());
                    paths.extend(tree.obtener_paths_hijos());
                }
            }
        }
        paths
    }

    /// Dado el contenido del arbol, devuelve un vector con los datos de cada uno de sus hijos.
    /// Cada dato es una tupla con el modo, el nombre y el hash del hijo.
    fn obtener_datos_de_contenido(
        contenido: &str,
    ) -> Result<Vec<(String, String, String)>, String> {
        let mut contenido_parseado: Vec<(String, String, String)> = Vec::new();
        let mut lineas = contenido.split('\0').collect::<Vec<&str>>();

        if lineas[0] == "tree 0" {
            return Ok(vec![]);
        }

        lineas.remove(0);
        let mut lineas_separadas: Vec<&str> = Vec::new();
        lineas_separadas.push(lineas[0]);
        let ultima_linea = lineas.pop().unwrap();
        lineas.iter().skip(1).for_each(|x| {
            let (hash, modo_y_nombre) = x.split_at(40);
            lineas_separadas.push(hash);
            lineas_separadas.push(modo_y_nombre);
        });
        lineas_separadas.push(ultima_linea);
        for i in (0..lineas_separadas.len()).step_by(2) {
            if i + 1 < lineas_separadas.len() {
                let linea = lineas_separadas[i].split_whitespace();
                let linea_spliteada = linea.clone().collect::<Vec<&str>>();

                let modo = linea_spliteada[0];
                let nombre = linea_spliteada[1];
                let tupla = (
                    modo.to_string(),
                    nombre.to_string(),
                    lineas_separadas[i + 1].to_string(),
                );
                contenido_parseado.push(tupla);
            } else {
                return Err("Error al parsear el contenido del tree".to_string());
            }
        }
        Ok(contenido_parseado)
    }

    /// Lee el objeto tree de la base de datos en base a un hash pasado por parametro junto con
    /// el directorio en el que se encuentra el tree y lo devuelve como un objeto Tree
    pub fn from_hash(hash: &str, directorio: PathBuf, logger: Arc<Logger>) -> Result<Tree, String> {
        let contenido = descomprimir_objeto(hash, ".gir/objects/")?;
        let contenido_parseado = Self::obtener_datos_de_contenido(&contenido)?;
        let mut objetos: Vec<Objeto> = Vec::new();

        for (modo, nombre, hash_hijo) in contenido_parseado {
            let mut ubicacion = format!("{}/{}", directorio.display(), nombre);
            if directorio == PathBuf::from(".") {
                ubicacion = nombre.clone()
            }

            match modo.as_str() {
                "100644" => {
                    let blob = Objeto::Blob(Blob {
                        nombre,
                        ubicacion: PathBuf::from(ubicacion),
                        hash: hash_hijo.to_string(),
                        logger: logger.clone(),
                    });
                    objetos.push(blob);
                }
                "40000" => {
                    let tree =
                        Self::from_hash(&hash_hijo, PathBuf::from(ubicacion), logger.clone())?;
                    objetos.push(Objeto::Tree(tree));
                }
                _ => {}
            }
        }

        Ok(Tree {
            directorio,
            objetos,
            logger,
        })
    }

    /// Devuelve el tamaño del contenido del tree.
    pub fn obtener_tamanio(&self) -> Result<usize, String> {
        let contenido = match Self::obtener_contenido(&self.objetos) {
            Ok(contenido) => contenido,
            Err(_) => return Err("No se pudo obtener el contenido del tree".to_string()),
        };
        Ok(contenido.len())
    }

    /// Devuelve si el arvol contiene un hijo con el mismo hash que el pasado por parametro.
    pub fn contiene_misma_version_hijo(&self, hash_hijo: &str, ubicacion_hijo: &Path) -> bool {
        for objeto in &self.objetos {
            match objeto {
                Objeto::Blob(blob) => {
                    if blob.hash == hash_hijo && blob.ubicacion == ubicacion_hijo {
                        return true;
                    }
                }
                Objeto::Tree(tree) => {
                    if tree.contiene_misma_version_hijo(hash_hijo, ubicacion_hijo) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Devuelve si el arbol contiene un hijo con el mismo nombre que el pasado por parametro.
    pub fn contiene_hijo_por_ubicacion(&self, ubicacion_hijo: PathBuf) -> bool {
        for objeto in &self.objetos {
            match objeto {
                Objeto::Blob(blob) => {
                    if blob.ubicacion == ubicacion_hijo.clone() {
                        return true;
                    }
                }
                Objeto::Tree(tree) => {
                    if tree.contiene_hijo_por_ubicacion(ubicacion_hijo.clone()) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Devuelve si el arbol contiene un hijo con el mismo directorio que el pasado por parametro.
    pub fn contiene_directorio(&self, directorio: &Path) -> bool {
        for objeto in &self.objetos {
            match objeto {
                Objeto::Blob(_) => {}
                Objeto::Tree(tree) => {
                    if tree.directorio == *directorio || tree.contiene_directorio(directorio) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Ordena los objetos del arbol alfabeticamente por directorio.
    pub fn ordenar_objetos_alfabeticamente(objetos: &[Objeto]) -> Vec<Objeto> {
        let mut objetos = objetos.to_owned();
        objetos.sort_by(|a, b| {
            let directorio_a = match a {
                Objeto::Blob(blob) => PathBuf::from(blob.nombre.clone()),
                Objeto::Tree(tree) => tree.directorio.clone(),
            };
            let directorio_b = match b {
                Objeto::Blob(blob) => PathBuf::from(blob.nombre.clone()),
                Objeto::Tree(tree) => tree.directorio.clone(),
            };
            directorio_a.cmp(&directorio_b)
        });
        objetos
    }

    /// Escribe el arbol junto a todos sus hijos en la base de datos.
    pub fn escribir_en_base(&self) -> Result<(), String> {
        let hash = self.obtener_hash()?;
        let ruta = format!(".gir/objects/{}/{}", &hash[..2], &hash[2..]);

        let contenido = Self::obtener_contenido(&self.objetos)?;

        let header = format!("tree {}\0", contenido.len());
        let contenido_completo = [header.as_bytes(), &contenido].concat();

        io::escribir_bytes(&ruta, comprimir_contenido_u8(&contenido_completo)?)?;

        for objeto in &self.objetos {
            match objeto {
                Objeto::Blob(blob) => {
                    HashObject {
                        logger: self.logger.clone(),
                        escribir: true,
                        ubicacion_archivo: blob.ubicacion.clone(),
                    }
                    .ejecutar()?;
                }
                Objeto::Tree(tree) => tree.escribir_en_base()?,
            };
        }

        Ok(())
    }

    /// Teniendo el contenido descomprimido pasado a String
    /// devuelve el contenido del arbol en un formato pretty print.
    pub fn rearmar_contenido_descomprimido(contenido: &str) -> Result<String, String> {
        let mut contenido_pretty: Vec<String> = Vec::new();
        let mut contenido_splitteado_null = contenido.split('\0').collect::<Vec<&str>>();
        contenido_pretty.push(contenido_splitteado_null[0].to_string() + " ");
        let ultimo_hash = match contenido_splitteado_null.pop() {
            Some(hash) => hash,
            None => return Err("Error al parsear el contenido del tree".to_string()),
        };
        contenido_splitteado_null.iter().skip(1).for_each(|x| {
            let (hash, modo_y_nombre) = x.split_at(40);
            contenido_pretty.push(hash.to_string() + "\n");
            contenido_pretty.push(modo_y_nombre.to_string() + " ");
        });
        contenido_pretty.push(ultimo_hash.to_string());
        Ok(contenido_pretty.concat())
    }

    /// Devuelve si el arbol esta vacio.
    pub fn es_vacio(&self) -> bool {
        if self.objetos.is_empty() {
            return true;
        }
        self.objetos.iter().all(|objeto| match objeto {
            Objeto::Blob(_) => false,
            Objeto::Tree(tree) => tree.es_vacio(),
        })
    }

    /// Dado un arbol y una ruta de un directorio, busca si la ruta esta dentro del arbol
    /// y en caso positivo, devuelve el arbol asociado a la ubicacion de ese directorio.
    pub fn recorrer_arbol_hasta_sub_arbol_buscado(
        direccion_hijo: &str,
        arbol: Tree,
    ) -> Result<Tree, String> {
        let path_hijo = PathBuf::from(direccion_hijo);
        for objeto in arbol.objetos {
            match objeto {
                Objeto::Tree(tree) => {
                    if tree.directorio == path_hijo {
                        return Ok(tree);
                    } else if esta_directorio_habilitado(&path_hijo, &vec![tree.directorio.clone()])
                    {
                        let tree_buscado =
                            Self::recorrer_arbol_hasta_sub_arbol_buscado(direccion_hijo, tree)?;
                        return Ok(tree_buscado);
                    }
                }
                _ => continue,
            }
        }
        Err(format!(
            "No se encontro el directorio {} dentro de los directorios trackeados",
            direccion_hijo
        ))
    }
}

impl Display for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.directorio.file_name() {
            Some(name) => name,
            None => return Err(std::fmt::Error),
        };
        let hash = match self.obtener_hash() {
            Ok(hash) => hash,
            Err(_) => return Err(std::fmt::Error),
        };
        let string = format!("40000 {} {}\n", hash, name.to_string_lossy());
        write!(f, "{}", string)
    }
}

#[cfg(test)]

mod tests {
    use serial_test::serial;

    use crate::tipos_de_dato::logger::Logger;
    use crate::tipos_de_dato::{objeto::Objeto, objetos::tree::Tree};
    use crate::utils::compresion::descomprimir_contenido_u8;
    use crate::utils::io;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// Dado un vector de objetos, devuelve el contenido del arbol en un formato pretty print.
    fn mostrar_contenido(objetos: &[Objeto]) -> Result<String, String> {
        let mut output = String::new();

        let objetos_ordenados = Tree::ordenar_objetos_alfabeticamente(objetos);

        for objeto in objetos_ordenados {
            let line = match objeto {
                Objeto::Blob(blob) => format!("100644 {}    {}\n", blob.nombre, blob.hash),
                Objeto::Tree(tree) => {
                    let name = match tree.directorio.file_name() {
                        Some(name) => name,
                        None => return Err("Error al obtener el nombre del directorio".to_string()),
                    };
                    let hash = tree.obtener_hash()?;
                    format!("40000 {}    {}\n", name.to_string_lossy(), hash)
                }
            };
            output.push_str(&line);
        }
        Ok(output)
    }

    #[test]
    #[serial]
    fn test01_test_obtener_hash() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tree_test01")).unwrap());
        let objeto =
            Objeto::from_directorio(PathBuf::from("test_dir/objetos"), None, logger).unwrap();

        if let Objeto::Tree(ref tree) = objeto {
            tree.escribir_en_base().unwrap();
        }
        let hash = objeto.obtener_hash();
        assert_eq!(hash, "1442e275fd3a2e743f6bccf3b11ab27862157179");
    }

    #[test]
    #[serial]
    fn test02_test_obtener_tamanio() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tree_test02")).unwrap());
        let objeto =
            Objeto::from_directorio(PathBuf::from("test_dir/muchos_objetos"), None, logger)
                .unwrap();
        let tamanio = objeto.obtener_tamanio().unwrap();
        assert_eq!(tamanio, 83);
    }

    #[test]
    #[serial]
    fn test03_test_mostrar_contenido() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tree_test03")).unwrap());

        let objeto =
            Objeto::from_directorio(PathBuf::from("test_dir/objetos"), None, logger).unwrap();

        if let Objeto::Tree(tree) = objeto {
            let contenido = mostrar_contenido(&tree.objetos).unwrap();
            assert_eq!(
                contenido,
                "100644 archivo.txt    2b824e648965b94c6c6b3dd0702feb91f699ed62\n"
            );
        } else {
            unreachable!()
        }
    }

    #[test]
    #[serial]
    fn test04_test_mostrar_contenido_recursivo() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tree_test04")).unwrap());

        let objeto = Objeto::from_directorio(PathBuf::from("test_dir/"), None, logger).unwrap();

        if let Objeto::Tree(tree) = objeto {
            let contenido = mostrar_contenido(&tree.objetos).unwrap();
            assert_eq!(
                contenido,
                "40000 muchos_objetos    896ca4eb090e033d16d4e9b1027216572ac3eaae\n40000 objetos    1442e275fd3a2e743f6bccf3b11ab27862157179\n"
            );
        } else {
            unreachable!()
        }
    }

    #[test]
    #[serial]
    fn test05_escribir_en_base() -> Result<(), String> {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tree_test05")).unwrap());

        let objeto =
            Objeto::from_directorio(PathBuf::from("test_dir/objetos"), None, logger).unwrap();
        if let Objeto::Tree(tree) = objeto {
            tree.escribir_en_base().unwrap();

            let contenido_leido =
                io::leer_bytes(".gir/objects/14/42e275fd3a2e743f6bccf3b11ab27862157179")?;

            let contenido_descomprimido = descomprimir_contenido_u8(&contenido_leido).unwrap();

            let contenido_esperado = [
                b"tree 39\0100644 archivo.txt\0".to_vec(),
                Tree::decode_hex("2b824e648965b94c6c6b3dd0702feb91f699ed62").unwrap(),
            ]
            .concat();

            assert_eq!(contenido_descomprimido, contenido_esperado);

            Ok(())
        } else {
            unreachable!();
        }
    }

    #[test]
    #[serial]
    fn test06_escribir_en_base_con_anidados() -> Result<(), String> {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tree_test06")).unwrap());

        let objeto = Objeto::from_directorio(PathBuf::from("test_dir"), None, logger).unwrap();
        if let Objeto::Tree(tree) = objeto {
            tree.escribir_en_base().unwrap();

            let contenido_leido =
                io::leer_bytes(".gir/objects/d1/bd5884df89a9734e3b0a4e7721a4802d85cce8").unwrap();
            let contenido_descomprimido = descomprimir_contenido_u8(&contenido_leido).unwrap();

            let contenido_esperado = [
                b"tree 75\040000 muchos_objetos\0".to_vec(),
                Tree::decode_hex("896ca4eb090e033d16d4e9b1027216572ac3eaae").unwrap(),
                b"40000 objetos\0".to_vec(),
                Tree::decode_hex("1442e275fd3a2e743f6bccf3b11ab27862157179").unwrap(),
            ]
            .concat();

            assert_eq!(contenido_descomprimido, contenido_esperado);

            Ok(())
        } else {
            unreachable!();
        }
    }

    #[test]
    #[serial]

    fn test07_contiene_hijo_por_ubicacion() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tree_test07")).unwrap());

        let tree = Tree::from_directorio(PathBuf::from("src"), None, logger).unwrap();

        assert!(tree.contiene_hijo_por_ubicacion(PathBuf::from("src/utils/io.rs")))
    }

    #[test]
    #[serial]

    fn test08_contiene_hijo_por_ubicacion_rec() {
        let logger = Arc::new(Logger::new(PathBuf::from("tmp/tree_test08")).unwrap());

        let tree = Tree::from_directorio(PathBuf::from("src"), None, logger).unwrap();

        assert!(tree.contiene_directorio(&PathBuf::from("src/tipos_de_dato")))
    }
}
