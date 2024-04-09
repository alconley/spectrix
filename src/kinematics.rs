use super::nuclear_data::MassMap;
use serde::{Deserialize, Serialize};

const C: f64 = 2.99792458e8; //speed of light in m/s
const QBRHO2P: f64 = C * 1.0e-9; //convert charge (in units of e) * B (kG (tesla)) * rho (cm) to momentum in MeV
const SPS_DISPERSION: f64 = 1.96; // x-position/rho
const SPS_MAGNIFICATION: f64 = 0.39; // in x-position
const SPS_DETECTOR_WIRE_DIST: f64 = 4.28625; //Distance between anode wires in SPS focal plane detector cm

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KineParameters {
    pub target_z: u32,
    pub target_a: u32,
    pub projectile_z: u32,
    pub projectile_a: u32,
    pub ejectile_z: u32,
    pub ejectile_a: u32,
    pub b_field: f64,       //kG
    pub sps_angle: f64,     //deg
    pub projectile_ke: f64, //MeV
}

impl Default for KineParameters {
    fn default() -> Self {
        KineParameters {
            target_z: 6,
            target_a: 12,
            projectile_z: 1,
            projectile_a: 2,
            ejectile_z: 1,
            ejectile_a: 1,
            b_field: 7.9,
            sps_angle: 37.0,
            projectile_ke: 16.0,
        }
    }
}

impl KineParameters {
    pub fn get_residual_z(&self) -> u32 {
        self.target_z + self.projectile_z - self.ejectile_z
    }

    pub fn get_residual_a(&self) -> u32 {
        self.target_a + self.projectile_a - self.ejectile_a
    }

    pub fn generate_rxn_eqn(&self, nuc_map: &MassMap) -> String {
        let targ_str = match nuc_map.get_data(&self.target_z, &self.target_a) {
            Some(data) => &data.isotope,
            None => "Invalid",
        };

        let proj_str = match nuc_map.get_data(&self.projectile_z, &self.projectile_a) {
            Some(data) => &data.isotope,
            None => "Invalid",
        };

        let eject_str = match nuc_map.get_data(&self.ejectile_z, &self.ejectile_a) {
            Some(data) => &data.isotope,
            None => "Invalid",
        };

        let resid_str = match nuc_map.get_data(&self.get_residual_z(), &self.get_residual_a()) {
            Some(data) => &data.isotope,
            None => "Invalid",
        };

        format!("{}({},{}){}", targ_str, proj_str, eject_str, resid_str)
    }
}

//Returns z-offset of focal plane in cm
fn calculate_z_offset(params: &KineParameters, nuc_map: &MassMap) -> Option<f64> {
    let target = match nuc_map.get_data(&params.target_z, &params.target_a) {
        Some(data) => data,
        None => return None,
    };

    println!("Target: {:?}", target);
    let projectile = match nuc_map.get_data(&params.projectile_z, &params.projectile_a) {
        Some(data) => data,
        None => return None,
    };
    let ejectile = match nuc_map.get_data(&params.ejectile_z, &params.ejectile_a) {
        Some(data) => data,
        None => return None,
    };
    let residual = match nuc_map.get_data(&params.get_residual_z(), &params.get_residual_a()) {
        Some(data) => data,
        None => return None,
    };

    let angle_rads = params.sps_angle.to_radians();
    let q_val = target.mass + projectile.mass - ejectile.mass - residual.mass;
    let term1 = (projectile.mass * ejectile.mass * params.projectile_ke).sqrt()
        / (ejectile.mass + residual.mass)
        * angle_rads.cos();
    let term2 = (params.projectile_ke * (residual.mass - projectile.mass) + residual.mass * q_val)
        / (ejectile.mass + residual.mass);

    let mut ejectile_ke = term1 + (term1 * term1 + term2).sqrt();
    if ejectile_ke.is_nan() {
        return None;
    }
    ejectile_ke *= ejectile_ke;

    let ejectile_p = (ejectile_ke * (ejectile_ke + 2.0 * ejectile.mass)).sqrt();
    let rho = ejectile_p / ((ejectile.z as f64) * params.b_field * QBRHO2P);
    let val = (projectile.mass * ejectile.mass * params.projectile_ke / ejectile_ke).sqrt();
    let k = val * angle_rads.sin() / (ejectile.mass + residual.mass - val * angle_rads.cos());
    Some(-1.0 * rho * SPS_DISPERSION * SPS_MAGNIFICATION * k)
}

//Calculate weights for correcting focal plane position for kinematic shift
//Returns tuple of weights where should be used like xavg = x1 * result.0 + x2 * result.1
pub fn calculate_weights(params: &KineParameters, nuc_map: &MassMap) -> Option<(f64, f64)> {
    let z_offset = match calculate_z_offset(params, nuc_map) {
        Some(z) => z,
        None => return None,
    };
    let w1 = 0.5 - z_offset / SPS_DETECTOR_WIRE_DIST;
    let w2 = 1.0 - w1;
    Some((w1, w2))
}
