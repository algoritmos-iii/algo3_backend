# algo3_backend

## Prerrequisitos 

Debes tener Rust instalado. Para instalarlo ejecutá los siguientes comandos.

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y

source "$HOME/.cargo/env"
```

## Build and Run

Ejecutar el siguiente comando compilará y ejecutará el binario que estará escuchando requests en el puerto `8080` en `localhost` por default (en un Docker Container).

```bash
make run_docker
```

La segunda directiva admite la opción `FROM` que por defecto es `8080` y representa al port que al que se hace forwarding.

Ejecutar el siguiente comando compilará y ejecutará el binario que estará escuchando requests en el puerto `8080` en `localhost` por default (localmente).

```bash
make run
```

La directiva de arriba admite las siguientes opciones:
- `DOMAIN`: el dominio en el que se alojará el backend. Es `http://127.0.0.1` por defecto (en un futuro `https`).
- `PORT`: el puerto en el que el backend escuchará requests. Es `8080` por defecto.

## Para probar

El `Makefile` dispone de las siguientes directivas para jugar un poco con la API:

```bash
make test_enqueue_help
make test_get_next
make test_dismiss 
make test_clear
make test_get_queue
```

Todas las directivas de arriba tienen como opciones opcionales `DOMAIN` y `PORT` exceptuando a las siguientes:

- `test_enqueue_help` admite opcionalmente la opción `GROUP` para indicar el grupo que pide ayuda. Por defecto es `0`.
- `test_get_next` admite opcionalmente la opción `HELPER` para indicar el ayudante que brinda la ayuda. Por defecto es `Ayudante`.
- `test_dismiss_help` admite opcionalmente la opción `GROUP` para indicar el grupo que desestima la ayuda. Por defecto es `0`.

El loggeo de ayudas pueden verlo en [esta spreadsheet](https://docs.google.com/spreadsheets/d/145qVyafYthG1dfCjbz-VcoABRqTkyGszqWK03Ax0L8A/edit#gid=0) (acordate de borrar el log).

## Para correr los tests

Para correr la suite de tests corré el siguiente comando en consola:

```bash
make test
```

## Convenciones de código

1. Las dependencias deben estar ordenadas alfabeticamente por tipo de dependencia:
    ```toml
    [dependencies.crate]

    [dev-dependencies.crate]
    ```
    Si se quisieran agregar features, binarios o libs estos deben ir al final estando primero los libs, luego los binarios y luego los features. Si se deben agregar otro tipo de labels deben ir al final.
2. Todos los endpoints deben ser testeados.
3. Al importar crates, ordénelos en 2 secciones alfabeticamente de la siguiente manera:
    ```rust
    // SECCIÓN 1: Nuestros paquetes primero (en orden alfabetico):
    use crate::Foo;

    // SECCIÓN 2: Otros paquetes (en orden alfabetico):
    use core::Foo;
    use serde::Bar;
    use std::Baz;
    ```
4. Cuando escribas tests, nombralos como `#[test] fn test_xx()`. Para funciones auxiliares, nombralas como `fn xx_test()`.
