use vek::{Aabb, Vec3};

// https://www.gamedev.net/tutorials/programming/general-and-gameplay-programming/swept-aabb-collision-detection-and-response-r3084/
// https://www.gamedev.net/tutorials/_/technical/game-programming/swept-aabb-collision-detection-and-response-r3084/
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepBox {
    pub collider: Aabb<f32>,
    pub velocity: Vec3<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
// A normal of zero and travel of zero means the movement is already perfectly aligned.
pub struct SweepTestResult {
    pub normal: Vec3<f32>,
    pub time: f32,
}

pub fn sweep_test(a: SweepBox, b: Aabb<f32>) -> Option<SweepTestResult> {
    fn calc_axis_abs(a_min: f32, a_max: f32, b_min: f32, b_max: f32, direction: f32) -> (f32, f32) {
        if direction > 0.0 {
            (b_min - a_max, b_max - a_min)
        } else {
            (b_max - a_min, b_min - a_max)
        }
    }
    fn calc_axis_abs_aabb(
        a: SweepBox,
        b: Aabb<f32>,
        velocity: Vec3<f32>,
        axis: usize,
    ) -> (f32, f32) {
        calc_axis_abs(
            a.collider.min[axis],
            a.collider.max[axis],
            b.min[axis],
            b.max[axis],
            velocity[axis],
        )
    }
    fn calc_axis_rel(enter: f32, exit: f32, velocity: f32) -> (f32, f32) {
        if velocity == 0.0 {
            (-f32::INFINITY, f32::INFINITY)
        } else {
            (enter / velocity, exit / velocity)
        }
    }
    let (x_abs_enter, x_abs_exit) = calc_axis_abs_aabb(a, b, a.velocity, 0);
    let (y_abs_enter, y_abs_exit) = calc_axis_abs_aabb(a, b, a.velocity, 1);
    let (z_abs_enter, z_abs_exit) = calc_axis_abs_aabb(a, b, a.velocity, 2);
    let (x_enter, x_exit) = calc_axis_rel(x_abs_enter, x_abs_exit, a.velocity.x);
    let (y_enter, y_exit) = calc_axis_rel(y_abs_enter, y_abs_exit, a.velocity.y);
    let (z_enter, z_exit) = calc_axis_rel(z_abs_enter, z_abs_exit, a.velocity.z);

    let entry_time = x_enter.max(y_enter).max(z_enter);
    let exit_time = x_exit.min(y_exit).min(z_exit);

    if entry_time > exit_time
        || (x_enter < 0.0 && y_enter < 0.0 && z_enter < 0.0)
        || x_enter > 1.0
        || y_enter > 1.0
        || z_enter > 1.0
    {
        return None;
    }

    fn norm(v: f32, vel: f32) -> f32 {
        if v != 0.0 {
            -v.signum()
        } else {
            -vel.signum()
        }
    }

    let x_norm = norm(x_abs_enter, a.velocity.x);
    let y_norm = norm(y_abs_enter, a.velocity.y);
    let z_norm = norm(z_abs_enter, a.velocity.z);

    let normal = if x_enter > y_enter && x_enter > z_enter {
        Vec3::new(x_norm, 0.0, 0.0)
    } else if y_enter > x_enter && y_enter > z_enter {
        Vec3::new(0.0, y_norm, 0.0)
    } else if z_enter > x_enter && z_enter > y_enter {
        Vec3::new(0.0, 0.0, z_norm)
    }
    // Edges and corners
    else {
        // x,y edge
        if x_enter > z_enter && y_enter > z_enter {
            Vec3::new(x_norm, y_norm, 0.0).normalized()
        }
        // x,z edge
        else if x_enter > y_enter && z_enter > y_enter {
            Vec3::new(x_norm, 0.0, z_norm).normalized()
        }
        // y,z edge
        else if y_enter > x_enter && z_enter > x_enter {
            Vec3::new(0.0, y_norm, z_norm).normalized()
        }
        // x,y,z corner
        else {
            Vec3::new(x_norm, y_norm, z_norm).normalized()
        }
    };

    Some(SweepTestResult {
        normal,
        time: entry_time,
    })
}

// TODO use vek::Rect3 here

#[test]
pub fn test_sweep_test() {
    use vek::Extent3;

    assert_eq!(
        sweep_test(
            SweepBox {
                collider: Aabb {
                    min: Vec3::zero(),
                    max: Vec3::zero() + Extent3::one(),
                },
                velocity: Vec3::new(0.5, 0.0, 0.0),
            },
            Aabb {
                min: Vec3::new(2.0, 2.0, 0.0),
                max: Vec3::new(2.0, 2.0, 0.0) + Vec3::one()
            }
        ),
        None
    );
    assert_eq!(
        sweep_test(
            SweepBox {
                collider: Aabb {
                    min: Vec3::zero(),
                    max: Vec3::zero() + Extent3::one(),
                },
                velocity: Vec3::new(1.2, 0.0, 0.0),
            },
            Aabb {
                min: Vec3::new(2.0, 2.0, 0.0),
                max: Vec3::new(2.0, 2.0, 0.0) + Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3::new(-1.0, 0.0, 0.0),
            time: 0.8333333,
        })
    );

    assert_eq!(
        sweep_test(
            SweepBox {
                collider: Aabb {
                    min: Vec3::zero(),
                    max: Vec3::zero() + Extent3::one(),
                },
                velocity: Vec3::new(1.2, 1.3, 0.0),
            },
            Aabb {
                min: Vec3::new(2.0, 2.0, 0.0),
                max: Vec3::new(2.0, 2.0, 0.0) + Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: -1.0,
                y: 0.0,
                z: 0.0
            },
            time: 0.8333333
        })
    );

    assert_eq!(
        sweep_test(
            SweepBox {
                collider: Aabb {
                    min: Vec3::zero(),
                    max: Vec3::zero() + Extent3::one(),
                },
                velocity: Vec3::new(1.2, 1.2, 0.0),
            },
            Aabb {
                min: Vec3::new(2.0, 2.0, 0.0),
                max: Vec3::new(2.0, 2.0, 0.0) + Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: -0.70710677,
                y: -0.70710677,
                z: 0.0
            },
            time: 0.8333333
        })
    );

    assert_eq!(
        sweep_test(
            SweepBox {
                collider: Aabb {
                    min: Vec3::new(0.5, 1.5, 0.5),
                    max: Vec3::new(0.5, 1.5, 0.5) + Extent3::one(),
                },
                velocity: Vec3::new(0.0, -0.1, 0.0),
            },
            Aabb {
                min: Vec3::one(),
                max: Vec3::one() + Vec3::one()
            }
        ),
        None
    );
    assert_eq!(
        sweep_test(
            SweepBox {
                collider: Aabb {
                    min: Vec3::new(0.5, 1.1, 0.5),
                    max: Vec3::new(0.5, 1.1, 0.5) + Extent3::one(),
                },
                velocity: Vec3::new(0.0, -0.2, 0.0),
            },
            Aabb {
                min: Vec3::zero(),
                max: Vec3::zero() + Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0
            },
            time: 0.5000001
        })
    );
    assert_eq!(
        sweep_test(
            SweepBox {
                collider: Aabb {
                    min: Vec3::new(0.5, 2.1, 0.5),
                    max: Vec3::new(0.5, 2.1, 0.5) + Extent3::one(),
                },
                velocity: Vec3::new(0.0, -0.2, 0.0),
            },
            Aabb {
                min: Vec3::one(),
                max: Vec3::one() + Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0
            },
            time: 0.49999952
        })
    );
    assert_eq!(
        sweep_test(
            SweepBox {
                collider: Aabb {
                    min: Vec3::new(0.5, 1.1, 0.5),
                    max: Vec3::new(0.5, 1.1, 0.5) + Extent3::one(),
                },
                velocity: Vec3::new(0.0, -0.4, 0.0),
            },
            Aabb {
                min: Vec3::zero(),
                max: Vec3::zero() + Vec3::one()
            }
        ),
        Some(SweepTestResult {
            normal: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0
            },
            time: 0.25000006
        })
    );
}
