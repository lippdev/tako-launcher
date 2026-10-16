//! Formato KeyValues/VDF da Steam em texto (port de `Vdf.cs`): `"chave" "valor"` ou `"chave" { ... }`.
//! Chaves sem diferenciar maiúsculas; comentários `//` e condicionais `[$WIN32]` são ignorados.

/// Blocos mais fundos que isso são cortados (arquivo não confiável: sem estourar a pilha).
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum VdfValue {
    Text(String),
    Node(VdfNode),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VdfNode {
    /// Na ordem do arquivo; chave repetida troca o valor (como o dicionário do C#).
    pub children: Vec<(String, VdfValue)>,
}

impl VdfNode {
    fn get(&self, key: &str) -> Option<&VdfValue> {
        self.children
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    pub fn value(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            VdfValue::Text(s) => Some(s),
            VdfValue::Node(_) => None,
        }
    }

    pub fn node(&self, key: &str) -> Option<&VdfNode> {
        match self.get(key)? {
            VdfValue::Node(n) => Some(n),
            VdfValue::Text(_) => None,
        }
    }

    /// Caminho de nós, ex.: `path(&["UserLocalConfigStore", "Software", "Valve"])`.
    pub fn path(&self, keys: &[&str]) -> Option<&VdfNode> {
        keys.iter().try_fold(self, |n, k| n.node(k))
    }

    fn set(&mut self, key: String, value: VdfValue) {
        match self
            .children
            .iter_mut()
            .find(|(k, _)| k.eq_ignore_ascii_case(&key))
        {
            Some(slot) => slot.1 = value,
            None => self.children.push((key, value)),
        }
    }
}

/// Lê o documento inteiro; o resultado é o nó raiz (que contém o de nível superior, ex.: "AppState").
pub fn parse(text: &str) -> VdfNode {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0;
    let mut root = VdfNode::default();
    parse_into(&mut root, &chars, &mut pos, 0);
    root
}

/// Lê um arquivo (no máximo [`crate::MAX_STORE_FILE`]).
pub fn load(path: &std::path::Path) -> std::io::Result<VdfNode> {
    let bytes = crate::read_limited(path, crate::MAX_STORE_FILE)?;
    Ok(parse(&crate::text_of(&bytes)))
}

fn parse_into(node: &mut VdfNode, t: &[char], pos: &mut usize, depth: usize) {
    loop {
        let Some((key, key_quoted)) = next_token(t, pos) else {
            return;
        };
        if key == "}" && !key_quoted {
            return; // fim do bloco
        }
        let Some((value, value_quoted)) = next_token(t, pos) else {
            return;
        };
        skip_conditional(t, pos);
        if value == "{" && !value_quoted {
            let mut child = VdfNode::default();
            if depth < MAX_DEPTH {
                parse_into(&mut child, t, pos, depth + 1);
            } else {
                skip_block(t, pos);
            }
            node.set(key, VdfValue::Node(child));
        } else {
            node.set(key, VdfValue::Text(value));
        }
    }
}

/// Pula um bloco fundo demais sem recursão.
fn skip_block(t: &[char], pos: &mut usize) {
    let mut open = 1usize;
    while open > 0 {
        match next_token(t, pos) {
            None => return,
            Some((s, false)) if s == "{" => open += 1,
            Some((s, false)) if s == "}" => open -= 1,
            _ => {}
        }
    }
}

fn skip_conditional(t: &[char], pos: &mut usize) {
    let mut p = *pos;
    skip_space(t, &mut p);
    if t.get(p) == Some(&'[') {
        *pos = t[p..]
            .iter()
            .position(|&c| c == ']')
            .map_or(t.len(), |i| p + i + 1);
    }
}

fn next_token(t: &[char], pos: &mut usize) -> Option<(String, bool)> {
    skip_space(t, pos);
    let c = *t.get(*pos)?;
    if c == '{' || c == '}' {
        *pos += 1;
        return Some((c.to_string(), false));
    }
    let mut out = String::new();
    if c == '"' {
        *pos += 1;
        while let Some(&c) = t.get(*pos) {
            if c == '"' {
                break;
            }
            if c == '\\' && *pos + 1 < t.len() {
                *pos += 1;
                out.push(match t[*pos] {
                    'n' => '\n',
                    't' => '\t',
                    other => other,
                });
            } else {
                out.push(c);
            }
            *pos += 1;
        }
        *pos += 1; // aspas de fechamento
        return Some((out, true));
    }
    while let Some(&c) = t.get(*pos) {
        if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
            break;
        }
        out.push(c);
        *pos += 1;
    }
    Some((out, false))
}

fn skip_space(t: &[char], pos: &mut usize) {
    while let Some(&c) = t.get(*pos) {
        if c.is_whitespace() {
            *pos += 1;
        } else if c == '/' && t.get(*pos + 1) == Some(&'/') {
            *pos = t[*pos..]
                .iter()
                .position(|&c| c == '\n')
                .map_or(t.len(), |i| *pos + i + 1);
        } else {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_nodes_escapes_comments_and_conditionals() {
        let root = parse(
            r#"
            // comentário
            "AppState"
            {
                "appid"     "570"
                "name"      "Dota \"2\""
                "UserConfig" { "language" "brazilian" }
                "platform"  "win" [$WIN32]
            }
            "#,
        );
        let state = root.node("appstate").unwrap();
        assert_eq!(state.value("APPID"), Some("570"));
        assert_eq!(state.value("name"), Some("Dota \"2\""));
        assert_eq!(
            state.path(&["UserConfig"]).unwrap().value("language"),
            Some("brazilian")
        );
        assert_eq!(state.value("platform"), Some("win"));
    }

    #[test]
    fn broken_or_hostile_input_does_not_panic() {
        assert_eq!(parse("").children.len(), 0);
        let _ = parse("\"a\" { \"b\" \"unterminated");
        let deep = "\"a\" {".repeat(10_000);
        let root = parse(&deep);
        assert!(root.node("a").is_some());
        let _ = parse("[ } } { \\");
    }
}
