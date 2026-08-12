use std::{path::PathBuf, sync::Arc};

use nalgebra::Vector3;
use rand::Rng;

use crate::{arc_dyn, bvh::BVHNode, camera::{CameraBuilder, Render}, colour::Colour, hittable::{Hittable, HittableList, medium::ConstantMedium, quad::{Quad, quad_box}, rotate::rotate_y::RotateY, sphere::Sphere, translate::Translate, triangle::Triangle}, make_material, material::{Material, dielectric::Dialectric, diffuse_light::DiffuseLight, lambertian::Lambertian, metal::Metal}, texture::{Texture, barycentric_interpolation::BarycentricInterpolation, checker_texture::CheckerTexture, image_texture::ImageTexture, noise_texture::{MarbleTexture, NoiseTexture}, solid_colour::SolidColour}, vector::{V3, random_vector, random_vector_range}, world_add};


pub fn triangle(width: usize, height: usize) -> Render {
    let mut world = HittableList::new();

    let tex = arc_dyn!(BarycentricInterpolation::new(
        Colour::new(1.,0.,0.),
        Colour::new(0.,1.,0.),
        Colour::new(0.,0.,1.)
    ), Texture);

    let mat = make_material!(Lambertian (&tex));
    let triangle = Triangle::new(
        V3::new(-2.,-1.,0.),
        V3::new(2.,-1.,0.),
        V3::new(0.,2.,0.),
        &mat
    );
    world.add(Box::new(triangle));

    let camera = CameraBuilder::new()
        .with_aspect_ratio(1.)
        .with_background(V3::new(0.8,0.8,0.8))
        .with_width(width)
        .with_height(height)
        .with_max_depth(10)
        .with_samples_per_pixel(30)
        .with_camera_pos(
            V3::new(0.,0.,5.), 
            V3::new(0.,0.,0.), 
            V3::new(0.,1.,0.)
        )
        .build().unwrap();

    camera.render_parallel(&world)
}

fn _book2_final_scene(image_width: usize, samples_per_pixel: usize, max_depth: usize, ) -> Render {
    let mut boxes1 = HittableList::new();
    let mut rng = rand::rng();
    let ground = make_material!(Lambertian (0.48, 0.83, 0.53));

    let boxes_per_side = 20;
    for i in 0..boxes_per_side {
        for j in 0..boxes_per_side {
            let w = 100.;

            let x0 = -1000. + w * i as f64;
            let y0 = 0.;
            let z0 = -1000. + w * j as f64;

            let x1 = x0 + w;
            let y1 = rng.random_range(1.0 .. 101.);
            let z1 = z0 + w;

            let quad_box = Box::new(quad_box(V3::new(x0,y0,z0), V3::new(x1,y1,z1), &ground));
            boxes1.add(quad_box);
        }
    }

    let mut world = HittableList::new();
    world.add(Box::new(BVHNode::new(boxes1.objects)));

    let light = make_material!(DiffuseLight (7.,7.,7.));
    world_add!(Quad world, (123.,554.,147.), (300.,0.,0.), (0.,0.,265.), &light);

    let centre1 = V3::new(400., 400., 200.);
    let centre2 = centre1 + V3::new(30.,0.,0.);
    let sphere = Box::new(Sphere::new_moving(
        centre1, 
        centre2, 
        50., 
        &make_material!(Lambertian (0.7, 0.3, 0.1))
    ));
    world.add(sphere);

    world_add!(Sphere world, (260., 150., 45.), 50., &make_material!(Dialectric 1.5));
    world_add!(Sphere world, (0.,150.,145.), 50., &make_material!(Metal (0.8, 0.8, 0.9), 1.));

    let boundary = arc_dyn!(Sphere::new(V3::new(360.,150.,145.), 70., &make_material!(Dialectric 1.5)), Hittable);
    world.add(Box::new(Translate::new(&boundary, V3::new(0.,0.,0.))));
    world.add(Box::new(ConstantMedium::from_albedo(&boundary, 0.2, Colour::new(0.2, 0.4, 0.9))));
    let boundary = arc_dyn!(Sphere::new(V3::new(0.,0.,0.), 5000., &make_material!(Dialectric 1.5)), Hittable);
    world.add(Box::new(ConstantMedium::from_albedo(&boundary, 0.0005, Colour::new(1.,1.,1.))));

    let earthmap = arc_dyn!(ImageTexture::new(&PathBuf::from("images/earthmap.jpg")).unwrap(), Texture);
    let emat = make_material!(Lambertian (&earthmap));
    world_add!(Sphere world, (400.,200.,400.), 100., &emat);
    let pertext = arc_dyn!(NoiseTexture::new(0.2), Texture);
    world_add!(Sphere world, (220.,280.,300.), 80., &make_material!(Lambertian (&pertext)));

    let mut boxes2 = HittableList::new();
    let white = make_material!(Lambertian (0.73, 0.73, 0.73));
    let ns = 1000;
    for _ in 0..ns {
        boxes2.add(Box::new(Sphere::new(
            random_vector_range(&mut rng, 0., 165.),
            10.,
            &white
        )));
    }

    world.add(Box::new(Translate::new(
        &arc_dyn!(RotateY::new(
            &arc_dyn!(BVHNode::new(boxes2.objects), Hittable), 15.), Hittable),
            V3::new(-100.,270.,395.)
        ))
    );

    let cam = CameraBuilder::new()
        .with_aspect_ratio(1.)
        .with_width(image_width)
        .with_samples_per_pixel(samples_per_pixel)
        .with_max_depth(max_depth)
        .with_background(V3::new(0.,0.,0.))
        .with_vfov(40.)
        .with_camera_pos(
            V3::new(478.,278.,-600.), 
            V3::new(278.,278.,0.), 
            V3::new(0.,1.,0.))
        .build().unwrap();

    cam.render_parallel(&world)
}


//     camera cam;

//     cam.aspect_ratio      = 1.0;
//     cam.image_width       = image_width;
//     cam.samples_per_pixel = samples_per_pixel;
//     cam.max_depth         = max_depth;
//     cam.background        = color(0,0,0);

//     cam.vfov     = 40;
//     cam.lookfrom = point3(478, 278, -600);
//     cam.lookat   = point3(278, 278, 0);
//     cam.vup      = vec3(0,1,0);

//     cam.defocus_angle = 0;

//     cam.render(world);
// }

pub fn cornell_smoke() -> Render {
    let mut world = HittableList::new();

    let red = make_material!(Lambertian (0.65, 0.05, 0.05));
    let white = make_material!(Lambertian (0.73, 0.73, 0.73));
    let green = make_material!(Lambertian (0.12, 0.45, 0.15));
    let light = make_material!(DiffuseLight (7., 7., 7.));

    world_add!(Quad world, (555.,0.,0.), (0.,555.,0.), (0.,0.,555.), &green);
    world_add!(Quad world, (0.,0.,0.), (0.,555.,0.), (0.,0.,555.), &red);
    world_add!(Quad world, (113., 554., 127.), (330.,0.,0.), (0.,0.,305.), &light);
    world_add!(Quad world, (0.,0.,0.), (555.,0.,0.), (0.,0.,555.), &white);
    world_add!(Quad world, (555.,555.,555.), (-555.,0.,0.), (0.,0.,-555.), &white);
    world_add!(Quad world, (0.,0.,555.), (555.,0.,0.), (0.,555.,0.), &white);

    let box1 = arc_dyn!(quad_box(V3::new(0., 0.,0.), V3::new(165.,330.,165.), &white), Hittable);
    let box1 = arc_dyn!(RotateY::new(&box1, 15.), Hittable);
    let box1 = arc_dyn!(Translate::new(&box1, V3::new(265.,0.,295.)), Hittable);
    let box1 = Box::new(ConstantMedium::from_albedo(&box1, 0.01, Colour::new(0.,0.,0.)));
    world.add(box1);

    let box2 = arc_dyn!(quad_box(V3::new(0., 0.,0.), V3::new(165.,165.,165.), &white), Hittable);
    let box2 = arc_dyn!(RotateY::new(&box2, -18.), Hittable);
    let box2 = arc_dyn!(Translate::new(&box2, V3::new(130.,0.,65.)), Hittable);
    let box2 = Box::new(ConstantMedium::from_albedo(&box2, 0.01, Colour::new(1.,1.,1.)));
    world.add(box2);

    let camera = CameraBuilder::new()
        .with_aspect_ratio(1.)
        .with_width(600)
        .with_samples_per_pixel(200)
        .with_max_depth(50)
        .with_background(Colour::new(0.,0.,0.))
        .with_vfov(40.)
        // .with_background(Colour::new(0.5, 0.2, 0.))
        .with_camera_pos(
            V3::new(278., 278., -800.), 
            V3::new(278., 278., 0.), 
            V3::new(0.,1.,0.)
        ).build().unwrap();

    let world = BVHNode::new(world.objects);

    camera.render_parallel(&world)
}


fn _cornell_box() -> Render {
    let mut world = HittableList::new();

    let red = make_material!(Lambertian (0.65, 0.05, 0.05));
    let white = make_material!(Lambertian (0.73, 0.73, 0.73));
    let green = make_material!(Lambertian (0.12, 0.45, 0.15));
    let light = make_material!(DiffuseLight (15., 15., 15.));

    world_add!(Quad world, (555.,0.,0.), (0.,555.,0.), (0.,0.,555.), &green);
    world_add!(Quad world, (0.,0.,0.), (0.,555.,0.), (0.,0.,555.), &red);
    world_add!(Quad world, (343., 554., 332.), (-130.,0.,0.), (0.,0.,-105.), &light);
    world_add!(Quad world, (0.,0.,0.), (555.,0.,0.), (0.,0.,555.), &white);
    world_add!(Quad world, (555.,555.,555.), (-555.,0.,0.), (0.,0.,-555.), &white);
    world_add!(Quad world, (0.,0.,555.), (555.,0.,0.), (0.,555.,0.), &white);

    let box1 = arc_dyn!(quad_box(V3::new(0., 0.,0.), V3::new(165.,330.,165.), &white), Hittable);
    let box1 = arc_dyn!(RotateY::new(&box1, 15.), Hittable);
    let box1 = Box::new(Translate::new(&box1, V3::new(265.,0.,295.)));
    world.add(box1);

    let box2 = arc_dyn!(quad_box(V3::new(0., 0.,0.), V3::new(165.,165.,165.), &white), Hittable);
    let box2 = arc_dyn!(RotateY::new(&box2, -18.), Hittable);
    let box2 = Box::new(Translate::new(&box2, V3::new(130.,0.,65.)));
    world.add(box2);

    let camera = CameraBuilder::new()
        .with_aspect_ratio(1.)
        .with_width(600)
        .with_samples_per_pixel(200)
        .with_max_depth(50)
        .with_background(Colour::new(0.,0.,0.))
        .with_vfov(40.)
        // .with_background(Colour::new(0.5, 0.2, 0.))
        .with_camera_pos(
            V3::new(278., 278., -800.), 
            V3::new(278., 278., 0.), 
            V3::new(0.,1.,0.)
        ).build().unwrap();

    let world = BVHNode::new(world.objects);

    camera.render_parallel(&world)
}

fn _simple_light() -> Render {
    let mut world = HittableList::new();

    let pertext: Arc<dyn Texture> = Arc::new(MarbleTexture::new(4.));
    let mat1 = make_material!(Lambertian(&pertext));
    world_add!(Sphere world, (0.,-1000.,0.), 1000., &mat1);
    world_add!(Sphere world, (0.,2.,0.), 2., &mat1);

    let difflight = make_material!(DiffuseLight (4., 4., 4.));
    world_add!(Quad world, (3.,1.,-2.), (2.,0.,0.), (0.,2.,0.), &difflight);
    world_add!(Sphere world, (0.,7.,0.), 2., &difflight);

    let camera = CameraBuilder::new()
        .with_aspect_ratio(16. / 9.)
        .with_width(400)
        .with_samples_per_pixel(100)
        .with_max_depth(50)
        .with_vfov(20.)
        .with_camera_pos(
            V3::new(26., 3., 6.), 
            V3::new(0., 2., 0.), 
            V3::new(0., 1., 0.))
        .with_background(Colour::new(0.,0.,0.))
        .build().unwrap();

    let world = BVHNode::new(world.objects);
    camera.render_parallel(&world)
}

fn _quads() -> Render {
    let mut world = HittableList::new();

    let left_red     = make_material!(Lambertian(1.0, 0.2, 0.2));
    let back_green   = make_material!(Lambertian(0.2, 1.0, 0.2));
    let right_blue   = make_material!(Lambertian(0.2, 0.2, 1.0));
    let upper_orange = make_material!(Lambertian(1.0, 0.5, 0.0));
    let lower_teal   = make_material!(Lambertian(0.2, 0.8, 0.8));

    world_add!(Quad world, (-3.,-2.,5.), (0.,0.,-4.), (0.,4.,0.), &left_red);
    world_add!(Quad world, (-2.,-2.,0.), (4.,0.,0.),(0.,4.,0.), &back_green);
    world_add!(Quad world, (3.,-2.,1.), (0.,0.,4.), (0.,4.,0.), &right_blue);
    world_add!(Quad world, (-2.,3.,1.), (4.,0.,0.), (0.,0.,4.), &upper_orange);
    world_add!(Quad world, (-2.,-3.,5.), (4.,0.,0.), (0.,0.,-4.), &lower_teal);
    
    let camera = CameraBuilder::new()
        .with_aspect_ratio(1.)
        .with_width(400)
        .with_samples_per_pixel(100)
        .with_max_depth(50)
        .with_vfov(80.)
        .with_camera_pos(
            V3::new(0., 0., 9.), 
            V3::new(0., 0., 0.), 
            V3::new(0., 1., 0.))
        .build().unwrap();

    let world = BVHNode::new(world.objects);
    camera.render_parallel(&world)
}

fn _perlin_spheres() -> Render {
    let mut world = HittableList::new();

    let noise: Arc<dyn Texture> = Arc::new(MarbleTexture::new(4.));
    let material: Arc<dyn Material> = Arc::new(Lambertian::new(&noise));
    world.add(Box::new(Sphere::new(V3::new(0.,-1000.,0.), 1000., &material)));
    world.add(Box::new(Sphere::new(V3::new(0.,2.,0.), 2., &material)));

    let camera = CameraBuilder::new()
        .with_aspect_ratio(16. / 9.)
        .with_width(400)
        .with_samples_per_pixel(100)
        .with_max_depth(50)
        .with_vfov(20.)
        .with_camera_pos(
            V3::new(13., 2., 3.), 
            V3::new(0., 0., 0.), 
            V3::new(0., 1., 0.))
        .build().unwrap();

    camera.render_parallel(&world)
}

fn _earth() -> Render {
    let earth_texture = ImageTexture::new(&PathBuf::from("./images/earthmap.jpg")) .unwrap();
    let earth_texture: Arc<dyn Texture> = Arc::new(earth_texture);
    let earth_surface: Arc<dyn Material> = Arc::new(Lambertian::new(&earth_texture));
    let earth = Sphere::new(V3::new(0., 0., 0.), 2., &earth_surface);
    
    
    let camera = CameraBuilder::new()
        .with_aspect_ratio(16. / 9.)
        .with_width(400)
        .with_samples_per_pixel(100)
        .with_max_depth(50)
        .with_vfov(20.)
        .with_camera_pos(
            V3::new(0., 0., 12.), 
            V3::new(0., 0., 0.), 
            V3::new(0., 1., 0.))
        .build().unwrap();
    
    let mut world = HittableList::new();
    world.add(Box::new(earth));
    camera.render_parallel(&world)
}

fn _checkered_spheres() -> Render {
    let mut world = HittableList::new();

    let checker: Arc<dyn Texture> = Arc::new(CheckerTexture::new(
        0.32,
        Box::new(SolidColour::new(Colour::new(0.2, 0.3, 0.1))),
        Box::new(SolidColour::new(Colour::new(0.9, 0.9, 0.9))),
    ));
    let material: Arc<dyn Material> = Arc::new(Lambertian::new(&checker));
    
    world.add(Box::new(Sphere::new(V3::new(0., 10., 0.), 10., &material)));
    world.add(Box::new(Sphere::new(V3::new(0., -10., 0.), 10., &material)));

    let camera = CameraBuilder::new()
        .with_aspect_ratio(16. / 9.)
        .with_width(400)
        .with_samples_per_pixel(100)
        .with_max_depth(50)
        .with_vfov(20.)
        .with_camera_pos(
            V3::new(13., 2., 3.), 
            V3::new(0., 0., 0.), 
            V3::new(0., 1., 0.))
        .build().unwrap();

    let world = BVHNode::new(world.objects);

    camera.render_parallel(&world)
}

fn _book1_final_scene() -> Render {
    let mut rng = rand::rng();
    let mut world = HittableList::new();

    // let ground_material: Arc<dyn Material> = Arc::new(Lambertian::from_rgb(0.5, 0.5, 0.5));
    // world.add(Box::new(Sphere::new(V3::new(0.,-1000.,0.), 1000., &ground_material)));
    let checker: Arc<dyn Texture> = Arc::new(CheckerTexture::new(
        0.32, 
        Box::new(SolidColour::new(Colour::new(0.2, 0.3, 0.1))),
        Box::new(SolidColour::new(Colour::new(0.9, 0.9, 0.9)))));
    let material: Arc<dyn Material> = Arc::new(Lambertian::new(&checker));
    world.add(Box::new(Sphere::new(V3::new(0.,-1000.,0.), 1000., &material)));

    for a in -11..11 {
        for b in -11..11 {
            let choose_mat: f64 = rand::random();
            let centre: Vector3<f64> = V3::new(a as f64 + 0.9*rand::random::<f64>(), 0.2, b as f64 + 0.9*rand::random::<f64>()) ;

            if (centre - V3::new(4.,0.2,0.)).norm() > 0.9 {
                if choose_mat < 0.8 {
                    let albedo = random_vector(&mut rng).component_mul(&random_vector(&mut rng));
                    let sphere_material: Arc<dyn Material> = Arc::new(Lambertian::from_solid(albedo));
                    let centre2 = centre + random_vector_range(&mut rng, 0., 0.5);
                    world.add(Box::new(Sphere::new_moving(
                        centre,
                        centre2,
                        0.2, 
                        &sphere_material
                    )));
                } 
                else if choose_mat < 0.95 {
                    let albedo = random_vector_range(&mut rng, 0.5, 1.);
                    let fuzz = rng.random_range(0.0..0.5);
                    let sphere_material: Arc<dyn Material> = Arc::new(Metal::new(albedo, fuzz));
                    world.add(Box::new(Sphere::new(centre, 0.2, &sphere_material)));
                }
                else {
                    let sphere_material: Arc<dyn Material> = Arc::new(Dialectric::new(1.5));
                    world.add(Box::new(Sphere::new(centre, 0.2, &sphere_material)));
                }
            }
        }
    }

    let material1: Arc<dyn Material> = Arc::new(Dialectric::new(1.5));
    world.add(Box::new(Sphere::new(V3::new(0.,1.,0.), 1., &material1)));

    let material2: Arc<dyn Material> = Arc::new(Lambertian::from_rgb(0.4, 0.2, 0.1));
    world.add(Box::new(Sphere::new(V3::new(-4.,1.,0.), 1., &material2)));

    let material3: Arc<dyn Material> = Arc::new(Metal::from_rgb((0.7, 0.6, 0.5), 0.0));
    world.add(Box::new(Sphere::new(V3::new(4., 1., 0.), 1., &material3)));

    let camera = CameraBuilder::new()
        .with_aspect_ratio(16. / 9.)
        .with_width(800)
        .with_samples_per_pixel(400)
        .with_max_depth(100)
        .with_vfov(20.)
        .with_camera_pos(V3::new(13.,2.,3.), V3::new(0.,0.,0.), V3::new(0.,1.,0.))
        .with_defocus(10., 0.6)
        .build()
        .unwrap();

    let world = BVHNode::new(world.objects);
    camera.render_parallel(&world)
}