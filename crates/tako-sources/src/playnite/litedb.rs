//! Leitor, só de leitura, dos arquivos do LiteDB 4 (formato "v7", páginas de 4 KB) que o Playnite usa
//! na pasta `library` (port de `LiteDbV7Reader.cs` + BSON). Lê os documentos de uma coleção pelas
//! páginas do índice `_id`. Arquivo de outro formato (LiteDB 5, com senha) ou corrompido é `Err`.
// Adaptado de mbdavid/LiteDB: LiteDB/Engine/FileReader/FileReaderV7.cs (MIT), via
// JosefNemec/Playnite: source/Playnite/Database/Collections/LiteDBFileReaderV7.cs (MIT).

use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

const PAGE_SIZE: usize = 4096;
const HEADER_INFO: &[u8] = b"** This is a LiteDB file **";
const HEADER_PAGE: u8 = 1;
const COLLECTION_PAGE: u8 = 2;
const INDEX_PAGE: u8 = 3;
const DATA_PAGE: u8 = 4;
const EXTEND_PAGE: u8 = 5;
const EMPTY: u32 = u32::MAX;
/// Documentos dentro de documentos além disso são recusados (arquivo não confiável).
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct InvalidData(pub &'static str);

const TRUNCATED: InvalidData = InvalidData("Arquivo do LiteDB truncado ou corrompido");

/// Valor BSON.
#[derive(Debug, Clone, PartialEq)]
pub enum Bson {
    Double(f64),
    String(String),
    Document(Document),
    Array(Vec<Bson>),
    Binary(Vec<u8>),
    Guid(Uuid),
    ObjectId([u8; 12]),
    Bool(bool),
    /// Milissegundos desde 1970 (UTC).
    DateTime(i64),
    Int32(i32),
    Int64(i64),
    /// null, MinKey, MaxKey e decimal (nenhum campo usado tem).
    Null,
}

pub type Document = BTreeMap<String, Bson>;

/// Leitura little-endian com limite: passar do fim é `Err`.
struct Bytes<'a> {
    buf: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Bytes<'a> {
    fn new(buf: &'a [u8], start: usize, len: usize) -> Self {
        Bytes {
            buf,
            pos: start,
            end: (start + len).min(buf.len()),
        }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], InvalidData> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.end)
            .ok_or(TRUNCATED)?;
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn skip(&mut self, n: usize) -> Result<(), InvalidData> {
        self.take(n).map(drop)
    }

    fn u8(&mut self) -> Result<u8, InvalidData> {
        Ok(self.take(1)?[0])
    }

    fn arr<const N: usize>(&mut self) -> Result<[u8; N], InvalidData> {
        self.take(N)?.try_into().map_err(|_| TRUNCATED)
    }

    fn u16(&mut self) -> Result<u16, InvalidData> {
        Ok(u16::from_le_bytes(self.arr()?))
    }

    fn u32(&mut self) -> Result<u32, InvalidData> {
        Ok(u32::from_le_bytes(self.arr()?))
    }

    fn i32(&mut self) -> Result<i32, InvalidData> {
        Ok(i32::from_le_bytes(self.arr()?))
    }

    fn len(&mut self) -> Result<usize, InvalidData> {
        usize::try_from(self.i32()?).map_err(|_| TRUNCATED)
    }

    fn text(&mut self, n: usize) -> Result<String, InvalidData> {
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }

    /// Texto com o tamanho (int32) na frente, como o LiteDB 4 grava nomes de coleção e de índice.
    fn length_string(&mut self) -> Result<String, InvalidData> {
        let n = self.len()?;
        self.text(n)
    }

    /// Texto terminado em zero (nomes de campos do BSON).
    fn cstring(&mut self) -> Result<String, InvalidData> {
        let rest = &self.buf[self.pos..self.end];
        let zero = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(InvalidData("Texto sem fim no arquivo do LiteDB"))?;
        let s = String::from_utf8_lossy(&rest[..zero]).into_owned();
        self.pos += zero + 1;
        Ok(s)
    }
}

struct Page<'a> {
    kind: u8,
    next: u32,
    items: u16,
    body: Bytes<'a>,
}

struct IndexNode {
    data_page: u32,
    data_index: u16,
    prev_page: u32,
    next_page: u32,
}

pub struct LiteDbV7Reader {
    file: Vec<u8>,
    collections: Vec<(String, u32)>,
}

impl LiteDbV7Reader {
    pub fn new(file: Vec<u8>) -> Result<Self, InvalidData> {
        let mut reader = LiteDbV7Reader {
            file,
            collections: Vec::new(),
        };
        let header = reader.page(0).ok_or(InvalidData("Arquivo vazio"))?;
        if header.kind != HEADER_PAGE {
            return Err(InvalidData("Não é um arquivo do LiteDB 4"));
        }
        let mut r = header.body;
        let info = r.take(27)?;
        let version = r.u8()?;
        if info != HEADER_INFO || version != 7 {
            return Err(InvalidData("Não é um arquivo do LiteDB 4 (formato v7)"));
        }
        r.skip(2 + 4 + 4 + 2 + 20)?; // ChangeID, FreeEmptyPageID, LastPageID, userVersion, senha
        if r.take(16)?.iter().any(|&b| b != 0) {
            return Err(InvalidData("Arquivo do LiteDB protegido por senha"));
        }
        let count = r.u8()?;
        let mut collections = Vec::new();
        for _ in 0..count {
            let name = r.length_string()?;
            let page = r.u32()?;
            collections.push((name, page));
        }
        reader.collections = collections;
        Ok(reader)
    }

    pub fn collections(&self) -> impl Iterator<Item = &str> {
        self.collections.iter().map(|(n, _)| n.as_str())
    }

    /// Documentos da coleção (vazio se ela não existe).
    pub fn documents(&self, collection: &str) -> Result<Vec<Document>, InvalidData> {
        let Some(&(_, page_id)) = self
            .collections
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(collection))
        else {
            return Ok(Vec::new());
        };
        let Some(page) = self.page(page_id).filter(|p| p.kind == COLLECTION_PAGE) else {
            return Ok(Vec::new());
        };
        // Nome, 12 bytes e 16 índices; o primeiro com nome é o `_id`, que aponta para todos os documentos.
        let mut r = page.body;
        r.length_string()?;
        r.skip(12)?;
        let mut head = None;
        for _ in 0..16 {
            let field = r.length_string()?;
            r.skip(1)?; // unique
            let head_page = r.u32()?;
            r.skip(2 + 6 + 4)?; // HeadNode, TailNode, FreeIndexPageID
            if !field.is_empty() {
                head = Some(head_page);
                break;
            }
        }
        let Some(start) = head else {
            return Ok(Vec::new());
        };
        let mut documents = Vec::new();
        let mut seen = HashSet::new();
        for index_page in self.index_pages(start)? {
            for node in self.index_nodes(index_page)? {
                if node.data_page == EMPTY || !seen.insert((node.data_page, node.data_index)) {
                    continue;
                }
                if let Some(data) = self.data_block(node.data_page, node.data_index)?
                    && !data.is_empty()
                {
                    documents.push(read_document(&data)?);
                }
            }
        }
        Ok(documents)
    }

    fn page(&self, id: u32) -> Option<Page<'_>> {
        let offset = (id as usize).checked_mul(PAGE_SIZE)?;
        if offset.checked_add(PAGE_SIZE)? > self.file.len() {
            return None;
        }
        let mut r = Bytes::new(&self.file, offset, PAGE_SIZE);
        let parse = |r: &mut Bytes| -> Result<(u8, u32, u16), InvalidData> {
            r.u32()?; // pageID
            let kind = r.u8()?;
            r.u32()?; // anterior
            let next = r.u32()?;
            let items = r.u16()?;
            r.skip(2 + 8)?; // bytes livres e reservado
            Ok((kind, next, items))
        };
        let (kind, next, items) = parse(&mut r).ok()?;
        Some(Page {
            kind,
            next,
            items,
            body: r,
        })
    }

    /// Todas as páginas do índice, seguindo os nós anterior/próximo a partir da primeira.
    fn index_pages(&self, start: u32) -> Result<Vec<u32>, InvalidData> {
        let mut visited = Vec::new();
        let mut seen = HashSet::new();
        let mut pending = vec![start];
        while let Some(id) = pending.pop() {
            if id == EMPTY
                || seen.contains(&id)
                || !self.page(id).is_some_and(|p| p.kind == INDEX_PAGE)
            {
                continue;
            }
            seen.insert(id);
            visited.push(id);
            for node in self.index_nodes(id)? {
                pending.push(node.prev_page);
                pending.push(node.next_page);
            }
        }
        Ok(visited)
    }

    fn index_nodes(&self, page_id: u32) -> Result<Vec<IndexNode>, InvalidData> {
        let Some(page) = self.page(page_id).filter(|p| p.kind == INDEX_PAGE) else {
            return Ok(Vec::new());
        };
        let mut r = page.body;
        let mut nodes = Vec::new();
        for _ in 0..page.items {
            r.skip(2)?; // índice do nó
            let levels = usize::from(r.u8()?);
            r.skip(1 + 6 + 6)?; // slot, nó anterior e próximo
            let key_len = usize::from(r.u16()?);
            r.skip(1 + key_len)?; // tipo e valor da chave
            let data_page = r.u32()?;
            let data_index = r.u16()?;
            let prev_page = r.u32()?;
            r.skip(2)?;
            let next_page = r.u32()?;
            r.skip(2)?;
            r.skip(levels.saturating_sub(1) * (6 + 6))?;
            nodes.push(IndexNode {
                data_page,
                data_index,
                prev_page,
                next_page,
            });
        }
        Ok(nodes)
    }

    fn data_block(&self, page_id: u32, index: u16) -> Result<Option<Vec<u8>>, InvalidData> {
        let Some(page) = self.page(page_id).filter(|p| p.kind == DATA_PAGE) else {
            return Ok(None);
        };
        let mut r = page.body;
        for _ in 0..page.items {
            let block = r.u16()?;
            let extend = r.u32()?;
            let len = usize::from(r.u16()?);
            let data = r.take(len)?;
            if block == index {
                return Ok(Some(if extend == EMPTY {
                    data.to_vec()
                } else {
                    self.extend_data(extend)?
                }));
            }
        }
        Ok(None)
    }

    /// Documento grande: os bytes ficam numa corrente de páginas de extensão.
    fn extend_data(&self, mut page_id: u32) -> Result<Vec<u8>, InvalidData> {
        let mut out = Vec::new();
        let mut visited = HashSet::new();
        while page_id != EMPTY && visited.insert(page_id) {
            let Some(mut page) = self.page(page_id).filter(|p| p.kind == EXTEND_PAGE) else {
                return Ok(Vec::new());
            };
            out.extend_from_slice(page.body.take(usize::from(page.items))?);
            page_id = page.next;
        }
        Ok(out)
    }
}

// Adaptado de mbdavid/LiteDB: LiteDB/Document/Bson/BsonReader.cs (MIT), via JosefNemec/Playnite (MIT).
pub fn read_document(data: &[u8]) -> Result<Document, InvalidData> {
    let mut r = Bytes::new(data, 0, data.len());
    document(&mut r, 0)
}

fn document(r: &mut Bytes, depth: usize) -> Result<Document, InvalidData> {
    Ok(elements(r, depth)?.into_iter().collect())
}

fn elements(r: &mut Bytes, depth: usize) -> Result<Vec<(String, Bson)>, InvalidData> {
    if depth > MAX_DEPTH {
        return Err(InvalidData("Documento BSON fundo demais"));
    }
    let start = r.pos;
    let len = r.len()?;
    let end = (start + len).saturating_sub(1);
    let mut out = Vec::new();
    while r.pos < end {
        let kind = r.u8()?;
        let name = r.cstring()?;
        out.push((name, value(r, kind, depth)?));
    }
    r.skip(1)?; // zero final
    Ok(out)
}

fn value(r: &mut Bytes, kind: u8, depth: usize) -> Result<Bson, InvalidData> {
    Ok(match kind {
        0x01 => Bson::Double(f64::from_le_bytes(r.arr()?)),
        0x02 => {
            let n = r.len()?;
            let s = r.text(n.saturating_sub(1))?;
            r.skip(1)?;
            Bson::String(s)
        }
        0x03 => Bson::Document(document(r, depth + 1)?),
        0x04 => Bson::Array(
            elements(r, depth + 1)?
                .into_iter()
                .map(|(_, v)| v)
                .collect(),
        ),
        0x05 => {
            let n = r.len()?;
            let sub = r.u8()?;
            let bytes = r.take(n)?;
            match (sub, <[u8; 16]>::try_from(bytes)) {
                // Mesma ordem de bytes do `new Guid(bytes)` do .NET.
                (0x04, Ok(b)) => Bson::Guid(Uuid::from_bytes_le(b)),
                _ => Bson::Binary(bytes.to_vec()),
            }
        }
        0x07 => Bson::ObjectId(r.arr()?),
        0x08 => Bson::Bool(r.u8()? != 0),
        0x09 => Bson::DateTime(i64::from_le_bytes(r.arr()?)),
        0x0A | 0x7F | 0xFF => Bson::Null,
        0x10 => Bson::Int32(r.i32()?),
        0x12 => Bson::Int64(i64::from_le_bytes(r.arr()?)),
        0x13 => {
            r.skip(16)?;
            Bson::Null
        }
        _ => return Err(InvalidData("Tipo BSON não suportado")),
    })
}

/// Gera arquivos no formato do LiteDB 4 só para os testes (port de `tests/Shared/LiteDbV7Writer.cs`):
/// cabeçalho, uma página de coleção, o índice `_id`, páginas de dados e, para documentos grandes, a
/// corrente de páginas de extensão.
#[cfg(test)]
pub mod writer {
    use super::*;

    const HEADER_SIZE: usize = 25;
    const INLINE_LIMIT: usize = 2000;

    pub fn write(collections: &[(&str, Vec<Document>)]) -> Vec<u8> {
        let mut pages: Vec<Vec<u8>> = vec![vec![0; PAGE_SIZE]];
        let mut collection_pages = Vec::new();
        for (name, documents) in collections {
            let collection_page = add(&mut pages);
            let index_page = add(&mut pages);
            collection_pages.push((name.to_string(), collection_page));

            let mut locations: Vec<(u32, u16)> = Vec::new();
            let mut data_page = EMPTY;
            let mut used = 0;
            let mut blocks: Vec<Vec<u8>> = Vec::new();
            let flush = |pages: &mut Vec<Vec<u8>>, data_page: u32, blocks: &[Vec<u8>]| {
                if data_page != EMPTY {
                    write_page(
                        &mut pages[data_page as usize],
                        data_page,
                        DATA_PAGE,
                        EMPTY,
                        blocks.len() as u16,
                        &blocks.concat(),
                    );
                }
            };
            for document in documents {
                let bson = bson(document);
                let inline: &[u8] = if bson.len() <= INLINE_LIMIT {
                    &bson
                } else {
                    &[]
                };
                let block_size = 2 + 4 + 2 + inline.len();
                if data_page == EMPTY || used + block_size > PAGE_SIZE - HEADER_SIZE {
                    flush(&mut pages, data_page, &blocks);
                    data_page = add(&mut pages);
                    used = 0;
                    blocks.clear();
                }
                let extend = if bson.len() <= INLINE_LIMIT {
                    EMPTY
                } else {
                    write_extend(&mut pages, &bson)
                };
                let count = blocks.len() as u16;
                let mut block = count.to_le_bytes().to_vec();
                block.extend(extend.to_le_bytes());
                block.extend((inline.len() as u16).to_le_bytes());
                block.extend_from_slice(inline);
                blocks.push(block);
                locations.push((data_page, count));
                used += block_size;
            }
            flush(&mut pages, data_page, &blocks);

            // Índice _id: nó de cabeça (sem dados) + um nó por documento, todos na mesma página.
            let mut all = vec![(EMPTY, 0u16)];
            all.extend(locations);
            let mut nodes = Vec::new();
            for (i, (page, index)) in all.iter().enumerate() {
                let mut node = vec![0u8; 37];
                node[0..2].copy_from_slice(&(i as u16).to_le_bytes());
                node[2] = 1; // níveis
                node[19..23].copy_from_slice(&page.to_le_bytes());
                node[23..25].copy_from_slice(&index.to_le_bytes());
                let prev = if i == 0 { EMPTY } else { index_page };
                node[25..29].copy_from_slice(&prev.to_le_bytes());
                node[29..31].copy_from_slice(&(i.saturating_sub(1) as u16).to_le_bytes());
                let next = if i == all.len() - 1 {
                    EMPTY
                } else {
                    index_page
                };
                node[31..35].copy_from_slice(&next.to_le_bytes());
                node[35..37].copy_from_slice(&((i + 1) as u16).to_le_bytes());
                nodes.extend(node);
            }
            write_page(
                &mut pages[index_page as usize],
                index_page,
                INDEX_PAGE,
                EMPTY,
                all.len() as u16,
                &nodes,
            );

            // Página da coleção: nome, 12 bytes e 16 índices (o primeiro é o _id).
            let mut body = length_string(name);
            body.extend([0u8; 12]);
            for i in 0..16 {
                body.extend(length_string(if i == 0 { "_id" } else { "" }));
                body.push(u8::from(i == 0));
                body.extend((if i == 0 { index_page } else { EMPTY }).to_le_bytes());
                body.extend([0u8; 12]);
            }
            write_page(
                &mut pages[collection_page as usize],
                collection_page,
                COLLECTION_PAGE,
                EMPTY,
                0,
                &body,
            );
        }

        let mut header = HEADER_INFO.to_vec();
        header.push(7);
        header.extend([0u8; 2 + 4 + 4 + 2 + 20 + 16]);
        header.push(collection_pages.len() as u8);
        for (name, page) in &collection_pages {
            header.extend(length_string(name));
            header.extend(page.to_le_bytes());
        }
        write_page(&mut pages[0], 0, HEADER_PAGE, EMPTY, 0, &header);
        pages.concat()
    }

    /// Um documento em BSON.
    pub fn bson(document: &Document) -> Vec<u8> {
        let mut body = Vec::new();
        for (name, value) in document {
            element(&mut body, name, value);
        }
        wrap(body)
    }

    fn wrap(body: Vec<u8>) -> Vec<u8> {
        let mut out = ((body.len() + 5) as i32).to_le_bytes().to_vec();
        out.extend(body);
        out.push(0);
        out
    }

    fn element(body: &mut Vec<u8>, name: &str, value: &Bson) {
        let kind = match value {
            Bson::Null => 0x0A,
            Bson::Double(_) => 0x01,
            Bson::String(_) => 0x02,
            Bson::Document(_) => 0x03,
            Bson::Array(_) => 0x04,
            Bson::Guid(_) | Bson::Binary(_) => 0x05,
            Bson::ObjectId(_) => 0x07,
            Bson::Bool(_) => 0x08,
            Bson::DateTime(_) => 0x09,
            Bson::Int32(_) => 0x10,
            Bson::Int64(_) => 0x12,
        };
        body.push(kind);
        body.extend(name.as_bytes());
        body.push(0);
        match value {
            Bson::Double(d) => body.extend(d.to_le_bytes()),
            Bson::String(s) => {
                body.extend(((s.len() + 1) as i32).to_le_bytes());
                body.extend(s.as_bytes());
                body.push(0);
            }
            Bson::Document(d) => body.extend(bson(d)),
            Bson::Guid(g) => {
                body.extend(16i32.to_le_bytes());
                body.push(0x04);
                body.extend(g.to_bytes_le());
            }
            Bson::Binary(b) => {
                body.extend((b.len() as i32).to_le_bytes());
                body.push(0);
                body.extend(b);
            }
            Bson::ObjectId(o) => body.extend(o),
            Bson::Bool(b) => body.push(u8::from(*b)),
            Bson::DateTime(ms) | Bson::Int64(ms) => body.extend(ms.to_le_bytes()),
            Bson::Int32(i) => body.extend(i.to_le_bytes()),
            Bson::Array(items) => {
                let mut inner = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    element(&mut inner, &i.to_string(), item);
                }
                body.extend(wrap(inner));
            }
            Bson::Null => {}
        }
    }

    fn write_extend(pages: &mut Vec<Vec<u8>>, data: &[u8]) -> u32 {
        let capacity = PAGE_SIZE - HEADER_SIZE;
        let ids: Vec<u32> = data.chunks(capacity).map(|_| add(pages)).collect();
        for (i, chunk) in data.chunks(capacity).enumerate() {
            let next = ids.get(i + 1).copied().unwrap_or(EMPTY);
            write_page(
                &mut pages[ids[i] as usize],
                ids[i],
                EXTEND_PAGE,
                next,
                chunk.len() as u16,
                chunk,
            );
        }
        ids[0]
    }

    fn add(pages: &mut Vec<Vec<u8>>) -> u32 {
        pages.push(vec![0; PAGE_SIZE]);
        (pages.len() - 1) as u32
    }

    fn write_page(page: &mut [u8], id: u32, kind: u8, next: u32, items: u16, body: &[u8]) {
        page[0..4].copy_from_slice(&id.to_le_bytes());
        page[4] = kind;
        page[5..9].copy_from_slice(&EMPTY.to_le_bytes());
        page[9..13].copy_from_slice(&next.to_le_bytes());
        page[13..15].copy_from_slice(&items.to_le_bytes());
        page[HEADER_SIZE..HEADER_SIZE + body.len()].copy_from_slice(body);
    }

    fn length_string(text: &str) -> Vec<u8> {
        let mut out = (text.len() as i32).to_le_bytes().to_vec();
        out.extend(text.as_bytes());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playnite::sample;

    #[test]
    fn v7_reader_returns_every_document_including_extended_ones() {
        let reader = LiteDbV7Reader::new(writer::write(&[("Game", sample::games())])).unwrap();
        assert_eq!(reader.collections().collect::<Vec<_>>(), ["Game"]);
        let docs = reader.documents("Game").unwrap();
        assert_eq!(docs.len(), 8);
        let longo = docs
            .iter()
            .find(|d| d.get("Name") == Some(&Bson::String("Descrição Longa".into())))
            .unwrap();
        let Some(Bson::String(desc)) = longo.get("Description") else {
            panic!("sem descrição");
        };
        assert_eq!(desc.len(), 9000); // documento em páginas de extensão
        let lanterna = docs
            .iter()
            .find(|d| d.get("_id") == Some(&Bson::Guid(sample::LANTERNA)))
            .unwrap();
        assert_eq!(lanterna.get("Playtime"), Some(&Bson::Int64(7200)));
        assert_eq!(
            lanterna.get("Added"),
            Some(&Bson::DateTime(sample::ms(2025, 3, 1, 12)))
        );
        assert!(reader.documents("Inexistente").unwrap().is_empty());
    }

    #[test]
    fn reader_rejects_other_formats_and_corruption() {
        assert!(LiteDbV7Reader::new(vec![0; 100]).is_err());
        let mut v5 = writer::write(&[("Game", vec![])]);
        v5[25 + 27] = 8; // versão do arquivo do LiteDB 5
        assert!(LiteDbV7Reader::new(v5).is_err());

        // Bytes trocados em todo lugar: erro ou menos documentos, nunca pânico.
        let good = writer::write(&[("Game", sample::games())]);
        for i in (0..good.len()).step_by(7) {
            let mut bad = good.clone();
            bad[i] ^= 0xFF;
            if let Ok(r) = LiteDbV7Reader::new(bad) {
                let _ = r.documents("Game");
            }
        }
        assert!(read_document(&[0xFF, 0xFF, 0xFF, 0x7F]).is_err());
    }
}
