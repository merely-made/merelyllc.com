use super::*;

/// The proof's dependency fold over `root`, as the S5 fold fact: `root` and
/// every active instance it reaches through `depends_on` relations, followed
/// transitively and in table order (Ruling 154), with `root` as the member
/// stand-in, the rule recorded (Ruling 151), and the host's label (Ruling
/// 155). `None` when `root` reaches nothing, so there is nothing to fold.
pub(crate) fn dependency_fold(
    snapshot: &SceneSnapshot,
    root: sceno::InstanceId,
    name: &str,
) -> Option<Fold> {
    snapshot.active_item(root)?;
    let mut members = vec![root];
    let mut seen = HashSet::from([root]);
    let mut next = 0;
    while let Some(&at) = members.get(next) {
        next += 1;
        for relation in snapshot.tables.relations.iter().flatten() {
            if relation.from == at
                && relation.kind.as_deref() == Some(FOLD_FAMILY)
                && snapshot.active_item(relation.to).is_some()
                && seen.insert(relation.to)
            {
                members.push(relation.to);
            }
        }
    }
    (members.len() > 1).then(|| Fold {
        members,
        stand_in: StandIn::Member(root),
        rule: Some(FoldRule::Descendants {
            root,
            family: FOLD_FAMILY.to_owned(),
            direction: FoldDirection::Outgoing,
        }),
        boundary: None,
        label: Some(format!("{name}'s dependencies")),
    })
}

/// The step that folds or unfolds `root`'s dependencies where `snapshot`
/// stands: a `TombstoneFold` when `root` stands in for an active fold, else an
/// `AddFold` of [`dependency_fold`] in the next fold slot. `None` when there
/// is nothing to fold, or when the fold would share a member with an active
/// fold, which the fact refuses (folds do not nest).
pub(crate) fn fold_operations(
    snapshot: &SceneSnapshot,
    root: sceno::InstanceId,
    name: &str,
) -> Option<(String, Vec<SceneOp>)> {
    let folds = snapshot.active_folds();
    if let Some((index, _)) = folds
        .iter()
        .find(|(_, fold)| fold.stand_in_member() == Some(root))
    {
        return Some((
            format!("Expand {name} dependencies"),
            vec![SceneOp::TombstoneFold { index: *index }],
        ));
    }
    let fold = dependency_fold(snapshot, root, name)?;
    let overlaps = folds.iter().any(|(_, active)| {
        active
            .members
            .iter()
            .any(|member| fold.members.contains(member))
    });
    if overlaps {
        return None;
    }
    Some((
        format!("Fold {name} dependencies"),
        vec![SceneOp::AddFold {
            index: FoldId(snapshot.tables.folds.len() as u32),
            value: fold,
        }],
    ))
}

pub(super) fn instance_for_source(
    snapshot: &SceneSnapshot,
    source_id: &str,
) -> Option<sceno::InstanceId> {
    snapshot
        .tables
        .items
        .iter()
        .enumerate()
        .find_map(|(index, item)| {
            let item = item.as_ref()?;
            let source = snapshot
                .tables
                .sources
                .get(item.source.0 as usize)?
                .as_ref()?;
            (source.id == source_id).then_some(sceno::InstanceId(index as u32))
        })
}
