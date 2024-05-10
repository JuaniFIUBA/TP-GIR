#[derive(Clone)]
/// Representa una region en un archivo con conflictos,
/// normal si no hay conflictos, o conflicto si hay conflictos,
/// donde el primer elemento de la tupla es el contenido del HEAD,
/// y el segundo elemento es el contenido entrante
#[derive(PartialEq)]
pub enum Region {
    Normal(String),
    Conflicto(String, String),
}

impl std::fmt::Debug for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Region::Normal(contenido) => write!(f, "Normal({})", contenido),
            Region::Conflicto(contenido_head, contenido_entrante) => {
                write!(f, "Conflicto({},{})", contenido_head, contenido_entrante)
            }
        }
    }
}
impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Region::Normal(contenido) => write!(f, "{}", contenido),
            Region::Conflicto(contenido_head, contenido_entrante) => {
                write!(
                    f,
                    "<<<<<< HEAD\n{}\n======\n{}\n>>>>>> Entrante",
                    contenido_head, contenido_entrante
                )
            }
        }
    }
}

/// De un vector de regiones unifica aquellos conflictos adyacentes
/// por ejemplo si se tiene [Normal("hola"), Conflicto("a", "b"), Conflicto("c", "d"), Normal("chau")]
/// devuelve [Normal("hola"), Conflicto("ac", "bd"), Normal("chau")]
pub fn unificar_regiones(regiones: Vec<Region>) -> Vec<Region> {
    let mut regiones_unificadas: Vec<Region> = Vec::new();
    let mut i = 0;

    while i < regiones.len() {
        match &regiones[i] {
            Region::Normal(_) => {
                regiones_unificadas.push(regiones[i].clone());
                i += 1;
            }
            Region::Conflicto(a, b) => {
                if (a, b) == (&"".to_string(), &"".to_string()) {
                    i += 1;
                    continue;
                }

                let mut j = i;
                let mut buffer_head = Vec::new();
                let mut buffer_entrante = Vec::new();
                while j < regiones.len() {
                    match &regiones[j] {
                        Region::Normal(_) => break,
                        Region::Conflicto(lado_head, lado_entrante) => {
                            if !lado_head.is_empty() {
                                buffer_head.push(lado_head.trim());
                            }
                            if !lado_entrante.is_empty() {
                                buffer_entrante.push(lado_entrante.trim());
                            }
                        }
                    }
                    j += 1;
                }
                regiones_unificadas.push(Region::Conflicto(
                    buffer_head.join("\n"),
                    buffer_entrante.join("\n"),
                ));
                i = j;
            }
        }
    }
    purgar_conflictos(regiones_unificadas)
}

/// Si se tienen regiones vacias las elimina, y si hay conflictos que terminan con la misma linea,
/// extrae la linea como una linea normal.
pub fn purgar_conflictos(regiones: Vec<Region>) -> Vec<Region> {
    let mut regiones_purgadas: Vec<Region> = Vec::new();

    for region in regiones {
        match region {
            Region::Normal(_) => regiones_purgadas.push(region),
            Region::Conflicto(head, entrante) => {
                if head == *"" && entrante == *"" {
                    continue;
                }

                let mut head_split = head.split('\n').collect::<Vec<&str>>();
                let mut entrante_split = entrante.split('\n').collect::<Vec<&str>>();

                let mut regiones_normales: Vec<Region> = Vec::new();

                while head_split.last() == entrante_split.last() {
                    let linea = head_split.pop().unwrap();
                    entrante_split.pop();
                    regiones_normales.push(Region::Normal(linea.to_string()));
                }

                regiones_purgadas.push(Region::Conflicto(
                    head_split.join("\n"),
                    entrante_split.join("\n"),
                ));

                regiones_purgadas.extend(regiones_normales);
            }
        }
    }

    regiones_purgadas
}
