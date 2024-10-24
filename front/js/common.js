// common.js

const CHUNK = 4096;
let LEFT_PANEL_ITEMS;
let CONTEXT_MENU;
let MAIN_PANEL;
let COVER;

let LOGIN_TIMEOUT;
let SOCKET;
let NEXT_REQ_NUM = 0;
let CAN_CLOSE_POPUP = false;
let USER_ID;
let USER_DATA = {};
let INVITE_ENTITY_ID;
let CALLBACKS = [];

let USERNAMES = {};

let SIDES;

const SIDE_L = 0;
const SIDE_R = 1;

function is_mobile() {
    return window.matchMedia("(max-width: 1400px)").matches;
}

function create(parent, tagName, classes) {
    let element = document.createElement(tagName);

    for (let i = 0; i < classes.length; i++) {
        element.classList.add(classes[i]);
    }

    if (parent) parent.appendChild(element);
    return element;
}

function find(id) {
    return document.getElementById(id);
}

function setHandler(id, event, callback) {
    find(id).addEventListener(event, callback);
}

function child_index(element) {
    let parent = element.parentElement;
    return Array.prototype.indexOf.call(parent.children, element);
}

async function get_username(user_id) {
    if (USERNAMES[user_id]) return USERNAMES[user_id];

    let [_, parameters] = await request('load-user-data', user_id);
    let [rev, public] = parameters;
    USERNAMES[user_id] = public.name;

    return public.name;
}

function input_press_enter(event) {
    if (event.key === 'Enter' && this.enter_target) {
        if (this.enter_target.tagName.toLowerCase() === 'input') {
            this.enter_target.focus();
        } else {
            this.enter_target.click();
        }
    }
}

function redirect_enter(element, target) {
    element.enter_target = target;
    element.addEventListener('keydown', input_press_enter);
}

function close_popup(event) {
    if (CAN_CLOSE_POPUP && event.target === COVER) {
        COVER.classList.remove('bg-lv0');
        COVER.classList.add('bg-hide');
        COVER.classList.add('hidden');
        for (let i = 0; i < COVER.childElementCount; i++) {
            COVER.children[i].classList.add('hidden');
        }
    }
}

function selectable_click() {
    for (let i = 0; i < this.neighbors.length; i++) {
        this.neighbors[i].classList.remove('selected');
    }

    this.classList.add('selected');
}

function init_selectable(neighbors) {
    for (let i = 0; i < neighbors.length; i++) {
        neighbors[i].neighbors = neighbors;
        neighbors[i].addEventListener('click', selectable_click);
    }
}

function collapse_click() {
    let left_panel = find('left-panel');
    let classes = left_panel.classList;
    classes.toggle('collapsed');

    let text = classes.contains('collapsed') ? '...' : 'Your Objects';
    find('collapse-button').innerText = text;
}

function datetime_string(timestamp_s) {
    let d = new Date(timestamp_s * 1000);

    let date = ('0' + d.getDate()).slice(-2);
    let month = ('0'+(d.getMonth()+1)).slice(-2);
    let year = d.getFullYear();
    let hour = ('0' + d.getHours()).slice(-2);
    let minute = ('0' + d.getMinutes()).slice(-2);

    return date + '/' + month + '/' + year + ' - ' + hour + ':' + minute;
}
