use super::*;

impl GraphPhysics {
    pub(super) fn apply_arrangement(
        &mut self,
        positions: &str,
        mobility: &str,
    ) -> Result<(), String> {
        if !matches!(mobility, "anchored" | "free") {
            return Err(format!("unsupported mobility {mobility}"));
        }
        let positions: Vec<GraphNodePosition> = serde_json::from_str(positions)
            .map_err(|error| format!("invalid arrangement positions: {error}"))?;
        if positions.len() != self.key_by_id.len() {
            return Err("arrangement did not position every graph node".to_owned());
        }

        let current = self.simulation.positions().collect::<HashMap<_, _>>();
        for key in self.id_by_key.keys().copied().collect::<Vec<_>>() {
            self.simulation.unpin(key);
        }
        let mut targets = Vec::with_capacity(positions.len());
        for position in positions {
            let key = self
                .key_by_id
                .get(&position.id)
                .copied()
                .ok_or_else(|| format!("arrangement contains unknown node {}", position.id))?;
            let target = if self.manually_pinned.contains(&position.id) {
                current
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| Point2D::new(position.x, position.y))
            } else {
                Point2D::new(position.x, position.y)
            };
            targets.push((key, target));
        }
        self.simulation.seed_positions(targets.iter().copied());
        self.simulation.set_anchor_force(match mobility {
            "anchored" => Some(
                AnchorSpring::new(
                    targets
                        .iter()
                        .map(|(key, point)| (*key, (point.x, point.y))),
                )
                .with_stiffness(13.0),
            ),
            _ => None,
        });
        for id in &self.manually_pinned {
            let Some(key) = self.key_by_id.get(id).copied() else {
                continue;
            };
            let point = current
                .get(&key)
                .copied()
                .or_else(|| {
                    targets
                        .iter()
                        .find(|(candidate, _)| *candidate == key)
                        .map(|(_, p)| *p)
                })
                .unwrap_or_else(Point2D::origin);
            self.simulation.pin(key, point);
        }
        self.mobility = mobility.to_owned();
        Ok(())
    }

    pub(super) fn apply_backdrop(&mut self, backdrop: &str, tangible: bool) -> Result<(), String> {
        match backdrop {
            "clear" | "ambient" => {
                self.simulation.clear_scene();
                self.simulation.set_scene_field(None);
                self.simulation.set_gravity((0.0, 0.0));
                self.simulation.set_nodes_tangible(false);
            }
            "props" => {
                self.simulation.load_scene(&sandbox_props_scene());
                self.simulation.set_nodes_tangible(tangible);
            }
            "field" => {
                self.simulation.load_scene(&seiche::whirlpool_scene());
                self.simulation.set_scene_field(Some(SceneField::Vortex {
                    center: (0.0, 0.0),
                    strength: 54.0,
                    inward: 18.0,
                }));
                self.simulation.set_nodes_tangible(tangible);
            }
            _ => return Err(format!("unsupported backdrop {backdrop}")),
        }
        Ok(())
    }

    pub(super) fn frame_json(&self) -> Result<String, String> {
        let mut nodes = self
            .simulation
            .positions()
            .filter_map(|(key, position)| {
                let id = self.id_by_key.get(&key)?.clone();
                Some(PhysicsNode {
                    pinned: self.is_pinned(&id),
                    id,
                    x: position.x,
                    y: position.y,
                })
            })
            .collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.id.cmp(&right.id));
        let props = self
            .simulation
            .scene_bodies()
            .map(|body| physics_prop(body.position, body.rotation, body.collider))
            .collect();
        serde_json::to_string(&PhysicsFrame {
            schema: "mer3ly.graph-physics-frame/v1",
            nodes,
            props,
            at_rest: self.simulation.is_at_rest(0.35),
        })
        .map_err(|error| format!("could not serialize physics frame: {error}"))
    }
}
