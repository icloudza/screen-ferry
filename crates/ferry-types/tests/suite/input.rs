use ferry_types::RelativeMotionAccumulator;

#[test]
fn preserves_subpixel_motion_until_an_integer_is_reached() {
    let mut motion = RelativeMotionAccumulator::default();
    assert_eq!(motion.update(-0.6, 0.25), (0, 0));
    assert_eq!(motion.update(-0.6, 0.25), (-1, 0));
    assert_eq!(motion.update(-0.25, 0.5), (0, 1));
}

#[test]
fn reversing_direction_uses_the_remaining_fraction() {
    let mut motion = RelativeMotionAccumulator::default();
    assert_eq!(motion.update(0.75, -0.75), (0, 0));
    assert_eq!(motion.update(-0.5, 0.5), (0, 0));
    assert_eq!(motion.update(-1.25, 1.25), (-1, 1));
}

#[test]
fn integer_motion_is_emitted_immediately() {
    let mut motion = RelativeMotionAccumulator::default();
    assert_eq!(motion.update(-12.0, 7.0), (-12, 7));
}

#[test]
fn non_finite_motion_is_discarded_without_poisoning_later_events() {
    let mut motion = RelativeMotionAccumulator::default();
    assert_eq!(motion.update(0.5, f64::NAN), (0, 0));
    assert_eq!(motion.update(0.5, 0.5), (1, 0));
    assert_eq!(motion.update(f64::INFINITY, 0.0), (0, 0));
    assert_eq!(motion.update(1.0, 0.0), (1, 0));
}
