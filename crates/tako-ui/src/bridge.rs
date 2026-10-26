//! Aplica o estado calculado em Rust às propriedades do tema sem recriar listas: modelos são atualizados no
//! lugar (linha a linha), então mover o foco não reinicia repetidores nem animações.

use slint::{Model, ModelRc, VecModel};
use slint_interpreter::{ComponentInstance, Struct, Value};

/// Junta `new` ao valor antigo, reaproveitando os modelos (`VecModel`) que já existem.
pub fn reconcile(old: &Value, new: Value) -> Value {
    match (old, new) {
        (Value::Struct(o), Value::Struct(n)) => {
            let mut out = Struct::default();
            for (k, v) in n.iter() {
                let merged = match o.get_field(k) {
                    Some(ov) => reconcile(ov, v.clone()),
                    None => owned(v.clone()),
                };
                out.set_field(k.to_owned(), merged);
            }
            Value::Struct(out)
        }
        (Value::Model(o), Value::Model(n)) => match o.as_any().downcast_ref::<VecModel<Value>>() {
            Some(vm) => {
                sync(vm, n.iter().collect());
                Value::Model(o.clone())
            }
            None => owned(Value::Model(n)),
        },
        (_, n) => owned(n),
    }
}

/// Deixa `vm` igual a `rows`, mexendo só no que mudou.
pub fn sync(vm: &VecModel<Value>, rows: Vec<Value>) {
    let old_len = vm.row_count();
    for (i, row) in rows.iter().enumerate() {
        if i < old_len {
            let Some(old) = vm.row_data(i) else { continue };
            let merged = reconcile(&old, row.clone());
            if merged != old {
                vm.set_row_data(i, merged);
            }
        } else {
            // Linha nova: modelos internos também viram VecModel, para as próximas atualizações.
            vm.push(owned(row.clone()));
        }
    }
    for i in (rows.len()..old_len).rev() {
        vm.remove(i);
    }
}

/// Troca qualquer modelo por um `VecModel` próprio (recursivo), que o `sync` sabe atualizar.
pub fn owned(v: Value) -> Value {
    match v {
        Value::Model(m) => Value::Model(ModelRc::new(VecModel::from(
            m.iter().map(owned).collect::<Vec<_>>(),
        ))),
        Value::Struct(s) => {
            let mut out = Struct::default();
            for (k, f) in s.iter() {
                out.set_field(k.to_owned(), owned(f.clone()));
            }
            Value::Struct(out)
        }
        other => other,
    }
}

/// Define a propriedade só se mudou; listas são atualizadas no lugar.
pub fn set(ui: &ComponentInstance, name: &str, value: Value) {
    let Ok(old) = ui.get_property(name) else {
        tracing::debug!("o tema não tem a propriedade {name}");
        return;
    };
    let merged = reconcile(&old, value);
    if merged != old
        && let Err(e) = ui.set_property(name, merged)
    {
        tracing::warn!("propriedade {name}: {e}");
    }
}

pub fn set_global(ui: &ComponentInstance, global: &str, name: &str, value: Value) {
    match ui.get_global_property(global, name) {
        Ok(old) if old == value => {}
        Ok(_) => {
            if let Err(e) = ui.set_global_property(global, name, value) {
                tracing::warn!("{global}.{name}: {e}");
            }
        }
        Err(_) => tracing::debug!("o tema não tem {global}.{name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::{ToValue, model, obj};

    #[test]
    fn sync_updates_in_place() {
        let vm = VecModel::from(vec![
            obj(&[("a", 1.to_value())]),
            obj(&[("a", 2.to_value())]),
        ]);
        sync(
            &vm,
            vec![
                obj(&[("a", 1.to_value())]),
                obj(&[("a", 3.to_value())]),
                obj(&[("a", 4.to_value())]),
            ],
        );
        assert_eq!(vm.row_count(), 3);
        sync(&vm, vec![obj(&[("a", 9.to_value())])]);
        assert_eq!(vm.row_count(), 1);
    }

    #[test]
    fn nested_models_are_kept() {
        let first = owned(obj(&[("rows", model(vec![1.to_value()]))]));
        let Value::Struct(s) = &first else { panic!() };
        let Some(Value::Model(inner)) = s.get_field("rows") else {
            panic!()
        };
        let second = reconcile(
            &first,
            obj(&[("rows", model(vec![1.to_value(), 2.to_value()]))]),
        );
        let Value::Struct(s2) = &second else { panic!() };
        let Some(Value::Model(inner2)) = s2.get_field("rows") else {
            panic!()
        };
        assert!(inner == inner2, "o modelo interno deve ser o mesmo");
        assert_eq!(inner2.row_count(), 2);
    }
}
