use super::geometry::Shape;

pub(crate) fn find_collisions(shapes: Vec<Option<Shape>>) -> Vec<Vec<usize>> {
    let mut total_collisions = vec![];

    // @TODO: use a faster collision detection algorithm
    for (i, shape) in shapes.iter().enumerate().filter_map(to_option) {
        let mut this_shape_collisions = vec![i];

        for (j, another_shape) in shapes.iter().enumerate().skip(i + 1).filter_map(to_option) {
            if Shape::overlaps(shape, another_shape) {
                this_shape_collisions.push(j);
            }
        }

        if this_shape_collisions.len() > 1 {
            total_collisions.push(this_shape_collisions);
        }
    }

    total_collisions
}

fn to_option<T>(t: (usize, &Option<T>)) -> Option<(usize, &T)> {
    t.1.as_ref().map(|v| (t.0, v))
}

#[test]
fn test_find_collisions() {
    use crate::gamestate::world::{World, WorldPosition};

    let empty: Vec<Vec<usize>> = vec![];

    fn origin(v: (f32, f32)) -> WorldPosition {
        let world = World::init(1.0);
        world.new_position(v.into())
    }

    assert_eq!(find_collisions(vec![]), empty);
    assert_eq!(
        find_collisions(vec![
            Some(Shape::Circle {
                origin: origin((0.0, 0.0)),
                radius: 20.
            }),
            Some(Shape::Circle {
                origin: origin((40.0, 0.0)),
                radius: 10.
            })
        ]),
        empty
    );
    assert_eq!(
        find_collisions(vec![
            Some(Shape::Circle {
                origin: origin((0.0, 0.0)),
                radius: 20.
            }),
            Some(Shape::Circle {
                origin: origin((40.0, 0.0)),
                radius: 10.
            }),
            Some(Shape::Circle {
                origin: origin((-20.0, 0.0)),
                radius: 20.
            })
        ]),
        vec![vec![0_usize, 2_usize]]
    );
    assert_eq!(
        find_collisions(vec![
            None,
            Some(Shape::Circle {
                origin: origin((0.0, 0.0)),
                radius: 20.
            }),
            Some(Shape::Circle {
                origin: origin((40.0, 0.0)),
                radius: 10.
            }),
            Some(Shape::Circle {
                origin: origin((-20.0, 0.0)),
                radius: 20.
            })
        ]),
        vec![vec![1_usize, 3_usize]]
    );
    assert_eq!(
        find_collisions(vec![
            None,
            Some(Shape::Circle {
                origin: origin((0.0, -40.0)),
                radius: 15.
            }),
            Some(Shape::Circle {
                origin: origin((0.0, 40.0)),
                radius: 15.
            }),
        ]),
        vec![vec![1_usize, 2_usize]]
    );
}
