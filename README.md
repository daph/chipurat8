# Chipurat8
A little CHIP8 emulator written in rust

Run it by specifying your favorite chip8 rom:
`cargo run -- --rom roms/space_invaders.ch8`

You can find public domain roms [here](https://github.com/JohnEarnest/chip8Archive)

# Screenshots
[![chipurat8-test.png](https://i.postimg.cc/rFMwhWMD/chipurat8-test.png)](https://postimg.cc/Cdr05RZ0)
[![chipurat8-si.png](https://i.postimg.cc/c4xd7SBY/chipurat8-si.png)](https://postimg.cc/jC1VKB9d)

# Keymap
The relevant keys are:

```
1 2 3 4
Q W E R
A S D F
Z X C V
```
which map to the ol' hex pad:
```
1 2 3 C
4 5 6 D
7 8 9 E
A 0 B F
```
