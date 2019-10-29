
#### 2019.10.27
- Startup time optimization
    - Ground is loaded on a second thread as well
    - debug startup: ~5s
    - release startup: ~1.9s
    
#### 2019.10.27
- Startup time optimization  
Sprites and models are loaded on a background thread asynchronously. 
    - old, debug mode: 24s
    - old, release mode: ~3.5s (max value 6.1s)
    - new, debug mode: 7.6s
    - new, release mode: 2.7s (fairly stable, no fluctuations)

#### 2019.10.24
- Red and blue colors for all classes  
![Palettes](https://trello-attachments.s3.amazonaws.com/558a94779b3b3c5d89efeaa6/5d3dad963f865934aa69f051/c22dd3a7eda670ad6b1268ff12697d54/image.png)
- added `init.cmd`: A script file whose lines are executed on startup via the console system.  
It makes it possible to bind commands to shortcuts (see next point)
- Key binding command, e.g.: ``bind_key alt+Num1 toggle_console``
- `KeyState`s in `HumanInputComponent` are stored in a fixed size array not a hashmap (the index is the scancode value, which is a value from 0 to 284, hashmap was unnecessary)
- ``config-runtime.toml`` were expanded with `execute_script` property. It is for executing more complex and multiline commands (e.g. for the screenshot above, I needed to call `set_job JOB_NAME` and `clone` commands for all the available classes).  
Each commands are executed in a single frame to avoid problems with the physics system.  
Will be removed soon because it is a quite hacky solution.

#### 2019.10.23
- Palettes  
![Palettes](https://trello-attachments.s3.amazonaws.com/558a94779b3b3c5d89efeaa6/5d3dad963f865934aa69f051/2e4b89ed1f83638bc885f9ee0bf215ef/image.png)
