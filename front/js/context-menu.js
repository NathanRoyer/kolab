// context-menu.js

function hide_context_menu(event) {
    if (event.target !== this) return;
    CONTEXT_MENU.classList.remove('appear');
    CONTEXT_MENU.parentElement.classList.add('hidden');
    CONTEXT_MENU.innerHTML = '';
}

async function context_menu_click(event) {
    let callback_or_map = CONTEXT_MENU.current_map[this.innerText];
    CONTEXT_MENU.user_path.push(this.innerText);

    if (typeof callback_or_map === 'object') {
        CONTEXT_MENU.event = event;
        CONTEXT_MENU.current_map = callback_or_map;
        show_context_menu_2();
    } else {
        let orig_target = CONTEXT_MENU.original_target;
        await callback_or_map.call(orig_target, event, this.innerText);
        CONTEXT_MENU.parentElement.classList.add('hidden');
        CONTEXT_MENU.innerHTML = '';
    }
}

function show_context_menu(event) {
    event.stopPropagation();

    CONTEXT_MENU.classList.add('appear');
    CONTEXT_MENU.user_path = [];
    CONTEXT_MENU.event = event;
    CONTEXT_MENU.current_map = this.context_menu_map;
    CONTEXT_MENU.original_target = this;

    show_context_menu_2();
}

function show_context_menu_2() {
    CONTEXT_MENU.innerHTML = '';
    let event = CONTEXT_MENU.event;

    let labels = Object.keys(CONTEXT_MENU.current_map);
    for (let i = 0; i < labels.length; i++) {
        let not_last = (i + 1) < labels.length;
        let classes = ['pad05', 'btn', 'contained'];
        if (not_last) classes.push('border2-c1-bottom');
        let btn = create(CONTEXT_MENU, 'button', classes);
        btn.innerText = labels[i];
        btn.addEventListener('click', context_menu_click);
    }

    CONTEXT_MENU.style.setProperty('left', '-1000px');
    CONTEXT_MENU.style.setProperty('top', '-1000px');

    CONTEXT_MENU.parentElement.classList.remove('hidden');

    let width = CONTEXT_MENU.clientWidth;
    let height = CONTEXT_MENU.clientHeight;

    let invert_x = (event.clientX + width) > window.innerWidth;
    let invert_y = (event.clientY + height) > window.innerHeight;

    let left = event.clientX - (invert_x ? width : 0);
    let top = event.clientY - (invert_y ? height : 0);

    CONTEXT_MENU.style.setProperty('left', left.toString() + 'px');
    CONTEXT_MENU.style.setProperty('top', top.toString() + 'px');
}

function init_context_menu(element, map) {
    element.context_menu_map = map;
    element.addEventListener('click', show_context_menu);
}
