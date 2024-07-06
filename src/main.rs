use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    input::ButtonInput,
    log::LogPlugin,
    prelude::*,
    reflect::TypePath,
    window::PrimaryWindow,
};
use bevy_utils::BoxedFuture;
use serde::Deserialize;

const GAME_SCALE: f32 = 1.;
const BLOCK_SIZE: f32 = GAME_SCALE * 30.;
const MAP_SIZE: usize = 20;
const GAME_TITLE: &str = "Push Box Game";
const GAME_MENU_WIDTH: f32 = GAME_SCALE * 200.;
//const GAME_MENU_HEIGHT: f32 = GAME_SCALE * 600.;
const GAME_BUTTON_WIDTH: f32 = GAME_MENU_WIDTH / 2. * 0.88;
const GAME_BUTTON_HEIGHT: f32 = GAME_BUTTON_WIDTH * 0.382;
const GAME_BUTTON_INTERVAL: f32 = (GAME_MENU_WIDTH / 2. - GAME_BUTTON_WIDTH) / 3.;
const GAME_LEVEL_COUNT: usize = 50;
const INPUT_INTERVAL: f32 = 200.;

const MAP_SIZE_F32: f32 = MAP_SIZE as f32;
const GAME_WIDTH: f32 = BLOCK_SIZE * MAP_SIZE_F32 + GAME_MENU_WIDTH;
const GAME_HEIGHT: f32 = BLOCK_SIZE * MAP_SIZE_F32;
const GAME_TRANSFORM_X: f32 = -BLOCK_SIZE * (MAP_SIZE_F32 - 1.) / 2. - GAME_MENU_WIDTH / 2.;
const GAME_TRANSFORM_Y: f32 = -BLOCK_SIZE * (MAP_SIZE_F32 - 1.) / 2.;
const GAME_MENU_TRANSFORM_X: f32 = BLOCK_SIZE * MAP_SIZE_F32 / 2.;
const GAME_MENU_TRANSFORM_Y: f32 = 0.;

// block type
//const BLOCK_TYPE_BLANK: usize = 0;
//const BLOCK_TYPE_WALL: usize = 1;
const BLOCK_TYPE_GROUND: usize = 2;
const BLOCK_TYPE_BOX: usize = 3;
const BLOCK_TYPE_AIM: usize = 4;
const BLOCK_TYPE_PLAYER_DOWN: usize = 5;
const BLOCK_TYPE_PLAYER_RIGHT: usize = 6;
const BLOCK_TYPE_PLAYER_LEFT: usize = 7;
const BLOCK_TYPE_PLAYER_UP: usize = 8;
const BLOCK_TYPE_BOX_AIM: usize = 9;

const BUTTON_TEXT: [&str; 7] = ["PREV", "NEXT", "RESTART", "UP", "LEFT", "RIGHT", "DOWN"];
const BUTTON_POSITION: [[f32; 2]; 7] = [
    [
        -GAME_BUTTON_WIDTH / 2. - GAME_BUTTON_INTERVAL / 2.,
        2. * (GAME_BUTTON_HEIGHT + GAME_BUTTON_INTERVAL),
    ],
    [
        GAME_BUTTON_WIDTH / 2. + GAME_BUTTON_INTERVAL / 2.,
        2. * (GAME_BUTTON_HEIGHT + GAME_BUTTON_INTERVAL),
    ],
    [0., GAME_BUTTON_HEIGHT + GAME_BUTTON_INTERVAL],
    [0., -1. * (GAME_BUTTON_HEIGHT + GAME_BUTTON_INTERVAL)],
    [
        -GAME_BUTTON_WIDTH / 2. - GAME_BUTTON_INTERVAL / 2.,
        -2. * (GAME_BUTTON_HEIGHT + GAME_BUTTON_INTERVAL),
    ],
    [
        GAME_BUTTON_WIDTH / 2. + GAME_BUTTON_INTERVAL / 2.,
        -2. * (GAME_BUTTON_HEIGHT + GAME_BUTTON_INTERVAL),
    ],
    [0., -3. * (GAME_BUTTON_HEIGHT + GAME_BUTTON_INTERVAL)],
];
const BUTTON_KEY: [KeyCode; 7] = [
    KeyCode::KeyP,
    KeyCode::KeyN,
    KeyCode::KeyR,
    KeyCode::ArrowUp,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ArrowDown,
];

#[derive(Component)]
struct MapBlock {
    x: usize,
    y: usize,
}

#[derive(Component)]
struct ButtonTag;

#[derive(Resource, Default)]
struct StepIntervalTimer(Timer);

#[derive(Resource)]
struct ImageHandles {
    textures: Vec<Handle<Image>>,
}

#[derive(Resource)]
struct SoundHandle {
    sound: Handle<AudioSource>,
}

#[derive(Resource)]
struct MapHandle {
    map: Handle<MapAsset>,
}

#[derive(Asset, TypePath, Debug, Deserialize)]
struct MapAsset {
    #[allow(dead_code)]
    value: [[usize; MAP_SIZE]; MAP_SIZE],
    position: Vec2,
}

#[derive(Default)]
struct MapAssetsLoader;

impl AssetLoader for MapAssetsLoader {
    type Asset = MapAsset;
    type Settings = ();
    type Error = std::io::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut value = [[0usize; MAP_SIZE]; MAP_SIZE];
            let mut position = Vec2::ZERO;
            let mut text = String::from_utf8(bytes).unwrap();
            debug!("{:?}", text);

            text = text.replace("\r\n", "\n");

            for i in 0..MAP_SIZE {
                for j in 0..MAP_SIZE {
                    let index = i * (MAP_SIZE + 1) + j;
                    value[j][MAP_SIZE - i - 1] =
                        (text[index..(index + 1)]).parse::<usize>().unwrap_or(0);
                    if value[j][MAP_SIZE - i - 1] == BLOCK_TYPE_PLAYER_DOWN {
                        position.x = j as f32;
                        position.y = (MAP_SIZE - i - 1) as f32;
                    }
                }
            }

            Ok(MapAsset { value, position })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}

#[derive(PartialEq)]
enum GameStatus {
    StartPlaying,
    Playing,
}

#[derive(Resource)]
struct Game {
    update: bool,
    level: usize,
    status: GameStatus,
    map: [[usize; MAP_SIZE]; MAP_SIZE],
    position: Vec2,
    position_type: usize,
    action: Option<KeyCode>,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            update: true,
            level: 1,
            status: GameStatus::StartPlaying,
            map: [[0; MAP_SIZE]; MAP_SIZE],
            position: Vec2::new(0., 0.),
            position_type: BLOCK_TYPE_GROUND,
            action: None,
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .init_asset::<MapAsset>()
        .init_asset_loader::<MapAssetsLoader>()
        .add_systems(Startup, resource_setup)
        .add_systems(Startup, menu_setup.after(resource_setup))
        .add_systems(Update, menu_update.after(menu_setup))
        .add_systems(Update, game_update.after(resource_setup))
        .add_systems(Update, keyboard_input.after(resource_setup))
        .run();
}

fn menu_setup(mut commands: Commands, imagehandles: Res<ImageHandles>) {
    commands.spawn(SpriteBundle {
        texture: imagehandles.textures[11].clone(),
        transform: Transform {
            translation: Vec3::new(GAME_MENU_TRANSFORM_X, GAME_MENU_TRANSFORM_Y, 0.),
            scale: Vec3::new(GAME_SCALE, GAME_SCALE, 1.),
            ..default()
        },
        ..default()
    });
    for i in 0..7usize {
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::GRAY,
                    custom_size: Some(Vec2::new(GAME_BUTTON_WIDTH, GAME_BUTTON_HEIGHT)),
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        GAME_MENU_TRANSFORM_X + BUTTON_POSITION[i][0],
                        GAME_MENU_TRANSFORM_Y + BUTTON_POSITION[i][1],
                        1.,
                    ),
                    ..default()
                },
                ..default()
            })
            .insert(ButtonTag)
            .with_children(|parent| {
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.3, 0.3, 0.3),
                        custom_size: Some(Vec2::new(
                            GAME_BUTTON_WIDTH - GAME_SCALE * 7.,
                            GAME_BUTTON_HEIGHT - GAME_SCALE * 7.,
                        )),
                        ..default()
                    },
                    ..default()
                });
                parent.spawn(Text2dBundle {
                    text: Text::from_section(
                        BUTTON_TEXT[i],
                        TextStyle {
                            font_size: GAME_SCALE * 20.,
                            ..default()
                        },
                    ),
                    ..default()
                });
            });
    }
}

fn menu_update(
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut game: ResMut<Game>,
    time: Res<Time>,
    mut timer: ResMut<StepIntervalTimer>,
) {
    if !timer.0.tick(time.delta()).finished() {
        return;
    }
    if mouse_input.just_pressed(MouseButton::Left) {
        for window in windows.iter() {
            if let Some(cursor_position) = window.cursor_position() {
                for i in 0..7usize {
                    let cursor_x = cursor_position.x - GAME_WIDTH / 2.;
                    let cursor_y = GAME_HEIGHT / 2. - cursor_position.y;
                    let x = [
                        GAME_MENU_TRANSFORM_X + BUTTON_POSITION[i][0] - GAME_BUTTON_WIDTH / 2.,
                        GAME_MENU_TRANSFORM_X + BUTTON_POSITION[i][0] + GAME_BUTTON_WIDTH / 2.,
                    ];
                    let y = [
                        GAME_MENU_TRANSFORM_Y + BUTTON_POSITION[i][1] - GAME_BUTTON_HEIGHT / 2.,
                        GAME_MENU_TRANSFORM_Y + BUTTON_POSITION[i][1] + GAME_BUTTON_HEIGHT / 2.,
                    ];
                    if cursor_x > x[0] && cursor_x < x[1] && cursor_y > y[0] && cursor_y < y[1] {
                        game.action = Some(BUTTON_KEY[i]);
                        timer.0.reset();
                        game.update();
                    }
                }
            }
        }
    }
}

fn resource_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    debug!("resource setup");
    commands.insert_resource(StepIntervalTimer(Timer::from_seconds(
        INPUT_INTERVAL / 1000.,
        TimerMode::Once,
    )));

    let mut textures = vec![];
    for i in 0..10 {
        textures.push(asset_server.load(format!("imgs/{}.png", i)));
    }
    textures.push(asset_server.load("imgs/backgroundImg.png"));
    textures.push(asset_server.load("imgs/toolImg.png"));
    commands.insert_resource(ImageHandles { textures });

    // map
    commands.insert_resource(MapHandle {
        map: asset_server.load("maps/1.map"),
    });

    // Sound
    let sound = asset_server.load("sounds/breakout_collision.ogg");
    commands.insert_resource(SoundHandle { sound });

    // window
    for mut window in windows.iter_mut() {
        window.title = GAME_TITLE.to_owned();
        window.resolution.set(GAME_WIDTH, GAME_HEIGHT);
        window.resizable = false;
    }

    commands.insert_resource(Game::default());
    commands.spawn(Camera2dBundle::default());
}

fn game_update(
    mut commands: Commands,
    mut game: ResMut<Game>,
    imagehandles: Res<ImageHandles>,
    maphandle: Res<MapHandle>,
    asset_server: Res<AssetServer>,
    map_assets: Res<Assets<MapAsset>>,
    mut query: Query<(Entity, &MapBlock, &mut Handle<Image>)>,
) {
    if !game.update {
        return;
    }
    debug!("game update");
    game.update = false;
    match game.status {
        GameStatus::StartPlaying => {
            debug!("Start Playing");
            match map_assets.get(&maphandle.map) {
                Some(map) => {
                    debug!("load map {}.map success:", game.level);
                    game.map = map.value;
                    game.position = map.position;
                    for i in 0..MAP_SIZE {
                        for j in 0..MAP_SIZE {
                            let x = (i as f32) * BLOCK_SIZE + GAME_TRANSFORM_X;
                            let y = (j as f32) * BLOCK_SIZE + GAME_TRANSFORM_Y;
                            commands
                                .spawn(SpriteBundle {
                                    texture: imagehandles.textures[game.map[i][j]].clone(),
                                    transform: Transform {
                                        translation: Vec3::new(x, y, 0.),
                                        scale: Vec3::new(GAME_SCALE, GAME_SCALE, 1.),
                                        ..default()
                                    },
                                    ..default()
                                })
                                .insert(MapBlock { x: i, y: j });
                        }
                    }
                    game.status = GameStatus::Playing;
                }
                _ => {
                    debug!("load map {}.map error", game.level);
                    game.update = true;
                }
            }
        }
        GameStatus::Playing => {
            debug!("Playing Game");
            if !game.win() {
                for (_, mapblock, mut imagehandle) in query.iter_mut() {
                    if *imagehandle != imagehandles.textures[game.map[mapblock.x][mapblock.y]] {
                        *imagehandle =
                            imagehandles.textures[game.map[mapblock.x][mapblock.y]].clone();
                    }
                }
            } else {
                commands.insert_resource(MapHandle {
                    map: asset_server.load(format!("maps/{}.map", game.level)),
                });
                for (entity, _, _) in query.iter_mut() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut timer: ResMut<StepIntervalTimer>,
    mut game: ResMut<Game>,
    mut commands: Commands,
    sound: Res<SoundHandle>,
) {
    if !timer.0.tick(time.delta()).finished() {
        return;
    }
    if keyboard_input.any_pressed([
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
    ]) {
        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            game.action = Some(KeyCode::ArrowLeft);
            timer.0.reset();
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            game.action = Some(KeyCode::ArrowRight);
            timer.0.reset();
        }
        if keyboard_input.pressed(KeyCode::ArrowUp) {
            game.action = Some(KeyCode::ArrowUp);
            timer.0.reset();
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) {
            game.action = Some(KeyCode::ArrowDown);
            timer.0.reset();
        }
        commands.spawn(AudioBundle {
            source: sound.sound.clone(),
            settings: PlaybackSettings::DESPAWN,
        });
        game.update();
    }
}

impl Game {
    const fn get_player_type(action: Vec2) -> usize {
        match (action.x as i32, action.y as i32) {
            (1, 0) => BLOCK_TYPE_PLAYER_LEFT,
            (-1, 0) => BLOCK_TYPE_PLAYER_RIGHT,
            (0, 1) => BLOCK_TYPE_PLAYER_UP,
            (0, -1) => BLOCK_TYPE_PLAYER_DOWN,
            _ => BLOCK_TYPE_PLAYER_UP,
        }
    }
    fn step(&mut self, action: Vec2) {
        let next_position = self.position + action;
        debug!(
            "position:{},actioin:{},next_position:{}",
            self.position, action, next_position
        );
        if next_position.x.clamp(0., (MAP_SIZE - 1) as f32) != next_position.x
            || next_position.y.clamp(0., (MAP_SIZE - 1) as f32) != next_position.y
        {
            return;
        }
        // P _ -> X P,_
        // P A -> X P,A
        if self.map[next_position.x as usize][next_position.y as usize] == BLOCK_TYPE_GROUND
            || self.map[next_position.x as usize][next_position.y as usize] == BLOCK_TYPE_AIM
        {
            self.map[self.position.x as usize][self.position.y as usize] = self.position_type;
            self.position_type = self.map[next_position.x as usize][next_position.y as usize];
            self.map[next_position.x as usize][next_position.y as usize] =
                Self::get_player_type(action);
            self.position = next_position;
            self.update = true;
        }
        // P B _ -> X P,_ B
        // P B A -> X P,_ W
        // P W _ -> X P,A B
        // P W A -> X P,A W
        if self.map[next_position.x as usize][next_position.y as usize] == BLOCK_TYPE_BOX
            || self.map[next_position.x as usize][next_position.y as usize] == BLOCK_TYPE_BOX_AIM
        {
            let next2_position = next_position + action;
            if next2_position.x.clamp(0., (MAP_SIZE - 1) as f32) != next2_position.x
                || next2_position.y.clamp(0., (MAP_SIZE - 1) as f32) != next2_position.y
            {
                return;
            }
            if self.map[next2_position.x as usize][next2_position.y as usize] == BLOCK_TYPE_GROUND
                || self.map[next2_position.x as usize][next2_position.y as usize] == BLOCK_TYPE_AIM
            {
                self.map[self.position.x as usize][self.position.y as usize] = self.position_type;
                if self.map[next_position.x as usize][next_position.y as usize] == BLOCK_TYPE_BOX {
                    self.position_type = BLOCK_TYPE_GROUND;
                } else {
                    self.position_type = BLOCK_TYPE_AIM;
                }
                self.map[next_position.x as usize][next_position.y as usize] =
                    Self::get_player_type(action);
                if self.map[next2_position.x as usize][next2_position.y as usize]
                    == BLOCK_TYPE_GROUND
                {
                    self.map[next2_position.x as usize][next2_position.y as usize] = BLOCK_TYPE_BOX;
                } else {
                    self.map[next2_position.x as usize][next2_position.y as usize] =
                        BLOCK_TYPE_BOX_AIM;
                }
                self.position = next_position;
                self.update = true;
            }
        }
    }
    fn win(&mut self) -> bool {
        for i in 0..MAP_SIZE {
            for j in 0..MAP_SIZE {
                if self.map[i][j] == BLOCK_TYPE_BOX {
                    return false;
                }
            }
        }
        self.level = self.level + 1;
        if self.level > GAME_LEVEL_COUNT {
            self.level = 1;
        }
        self.update = true;
        self.status = GameStatus::StartPlaying;
        return true;
    }
    fn update(&mut self) {
        if self.status != GameStatus::Playing {
            return;
        }
        if let Some(action) = self.action {
            match action {
                KeyCode::ArrowLeft => {
                    self.step(Vec2::new(-1., 0.));
                }
                KeyCode::ArrowRight => {
                    self.step(Vec2::new(1., 0.));
                }
                KeyCode::ArrowUp => {
                    self.step(Vec2::new(0., 1.));
                }
                KeyCode::ArrowDown => {
                    self.step(Vec2::new(0., -1.));
                }
                _ => {}
            }
        }
        self.action = None;
    }
}
