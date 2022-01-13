use crate::gamestate::Shape;

pub(crate) fn find_collisions(
    world_size: (f32, f32),
    shapes: Vec<Option<Shape>>,
) -> Vec<Vec<usize>> {
    let mut total_collisions = vec![];

    for (i, shape) in shapes.iter().enumerate().filter_map(to_option) {
        let mut this_shape_collisions = vec![i];

        for (j, another_shape) in shapes.iter().enumerate().skip(i + 1).filter_map(to_option) {
            if Shape::overlaps(shape, another_shape, world_size) {
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
    let empty: Vec<Vec<usize>> = vec![];
    let world_size = (20., 20.);

    assert_eq!(find_collisions(world_size, vec![]), empty);
    assert_eq!(
        find_collisions(
            world_size,
            vec![
                Some(Shape::Sphere {
                    origin: (0.0, 0.0, 0.0).into(),
                    radius: 2.
                }),
                Some(Shape::Sphere {
                    origin: (4.0, 0.0, 0.0).into(),
                    radius: 1.
                })
            ]
        ),
        empty
    );
    assert_eq!(
        find_collisions(
            world_size,
            vec![
                Some(Shape::Sphere {
                    origin: (0.0, 0.0, 0.0).into(),
                    radius: 2.
                }),
                Some(Shape::Sphere {
                    origin: (4.0, 0.0, 0.0).into(),
                    radius: 1.
                }),
                Some(Shape::Sphere {
                    origin: (-3.0, 0.0, 0.0).into(),
                    radius: 3.
                })
            ]
        ),
        vec![vec![0_usize, 2_usize]]
    );
    assert_eq!(
        find_collisions(
            world_size,
            vec![
                None,
                Some(Shape::Sphere {
                    origin: (0.0, 0.0, 0.0).into(),
                    radius: 2.
                }),
                Some(Shape::Sphere {
                    origin: (4.0, 0.0, 0.0).into(),
                    radius: 1.
                }),
                Some(Shape::Sphere {
                    origin: (-3.0, 0.0, 0.0).into(),
                    radius: 3.
                })
            ]
        ),
        vec![vec![1_usize, 3_usize]]
    );
    assert_eq!(
        find_collisions(
            world_size,
            vec![
                None,
                Some(Shape::Sphere {
                    origin: (0.0, -9.0, 0.0).into(),
                    radius: 2.
                }),
                Some(Shape::Sphere {
                    origin: (0.0, 9.0, 0.0).into(),
                    radius: 2.
                }),
            ]
        ),
        vec![vec![1_usize, 2_usize]]
    );
}
