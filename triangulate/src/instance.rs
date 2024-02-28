use crate::noradextensions::BetterAxis;
use norad::designspace::{Axis, DesignSpaceDocument, Instance};
use std::collections::BTreeMap;

pub fn instance_to_location(
    ds: &DesignSpaceDocument,
    instance: &Instance,
) -> BTreeMap<String, f32> {
    let axis_name_to_axis = ds
        .axes
        .iter()
        .map(|ax| (ax.name.clone(), ax))
        .collect::<BTreeMap<String, &Axis>>();

    instance
        .location
        .iter()
        .map(|d| {
            let axis = axis_name_to_axis.get(&d.name).expect("Unknown axis");
            (
                axis.tag.clone(),
                axis.designspace_to_userspace(d.xvalue.unwrap_or(0.0)),
            )
        })
        .collect()
}

pub fn find_instance_by_name<'a>(
    ds: &'a DesignSpaceDocument,
    instance: &str,
) -> Option<&'a Instance> {
    ds.instances
        .iter()
        .find(|&dsinstance| Some(instance) == dsinstance.name.as_deref())
}

pub fn find_instance_by_location<'a>(
    ds: &'a DesignSpaceDocument,
    location: &BTreeMap<String, f32>,
) -> Option<&'a Instance> {
    ds.instances
        .iter()
        .find(|&dsinstance| location == &instance_to_location(ds, dsinstance))
}

pub fn filename_for(instance: &Instance) -> Option<String> {
    let name = if instance.familyname.is_some() && instance.stylename.is_some() {
        let mut name = instance.familyname.clone().unwrap();
        name.push('-');
        name.push_str(&instance.stylename.clone().unwrap());
        Some(name)
    } else {
        instance.name.clone()
    };
    name.map(|mut x| {
        x.push_str(".ufo");
        x.replace(' ', "")
    })
}
