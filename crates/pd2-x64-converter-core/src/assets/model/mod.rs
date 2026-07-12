mod container;
mod geometry;
mod skin;

#[cfg(test)]
mod tests;

use std::borrow::Cow;

use crate::error::Result;
use crate::manifest::LayoutState;

use container::{
  GEOMETRY_TYPE, MODEL_TYPE, Section, TOPOLOGY_TYPE, convert_model_primitive, invalid,
  parse_sections, rebuild_model,
};
use geometry::{
  ModelLayout, convert_geometry, convert_topology, detect_layout, validate_geometry_trailers,
  validate_topologies,
};
use skin::validate_skin_bindings;

pub(crate) fn classify(data: &[u8], label: &str) -> Result<LayoutState> {
  let (_, layout) = inspect_model(data, label)?;
  Ok(match layout {
    ModelLayout::X32 => LayoutState::SupportedX32,
    ModelLayout::X64 => LayoutState::AlreadyX64,
  })
}

fn inspect_model<'a>(data: &'a [u8], label: &str) -> Result<(Vec<Section<'a>>, ModelLayout)> {
  let sections = parse_sections(data, label)?;
  let layout = detect_layout(&sections, label)?;
  validate_geometry_trailers(&sections, layout, label)?;
  validate_topologies(&sections, label)?;
  validate_skin_bindings(&sections, layout, label)?;
  Ok((sections, layout))
}

pub(crate) fn convert(data: &[u8], label: &str) -> Result<Vec<u8>> {
  let (sections, layout) = inspect_model(data, label)?;
  if layout == ModelLayout::X64 {
    return Ok(data.to_vec());
  }

  let out = rebuild_model(&sections, data.len(), label, |section| {
    Ok(match section.type_id {
      GEOMETRY_TYPE => convert_geometry(
        section.payload,
        &format!("{label}: Geometry {}", section.ref_id),
      )?
      .into(),
      TOPOLOGY_TYPE => convert_topology(
        section.payload,
        &format!("{label}: Topology {}", section.ref_id),
      )?
      .into(),
      MODEL_TYPE => convert_model_primitive(
        section.payload,
        &format!("{label}: model {}", section.ref_id),
      )?
      .into(),
      _ => Cow::Borrowed(section.payload),
    })
  })?;

  if classify(&out, &format!("{label}: converted"))? != LayoutState::AlreadyX64 {
    return Err(invalid(
      label,
      "converted model failed x64 structural verification",
    ));
  }
  Ok(out)
}

#[cfg(test)]
pub(crate) fn legacy_model(skinned: bool) -> Vec<u8> {
  tests::x32_model(skinned)
}
