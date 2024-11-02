// entities.js

async function get_image(entity_id) {
    if (USER_DATA.entity_map[entity_id] == undefined) {
        await load_user_data();
    }

    return USER_DATA.entity_map[entity_id].image.data;
}

async function refresh_left_panel() {
    let [c1, c2] = await get_image("user-" + USER_ID);
    set_user_tile(USER_DATA.public.name, USER_DATA.public.status, c1, c2);

    USERNAMES[USER_ID] = USER_DATA.public.name;

    let prev_scroll = LEFT_PANEL_ITEMS.scrollTop;
    LEFT_PANEL_ITEMS.innerHTML = '';
    let has_objects = false;
    let entity_ids = Object.keys(USER_DATA.secret.entities);
    for (let i = entity_ids.length - 1; i >= 0; i--) {
        let id = entity_ids[i];
        if (!id.startsWith('user-')) {
            has_objects = true;
            add_entity(id);
        }
    }

    if (!has_objects) {
        let placeholder_c = ['pad05', 'disabled', 'ta-center'];
        let placeholder = create(LEFT_PANEL_ITEMS, 'i', placeholder_c);
        placeholder.innerText = 'Create an object to get started!';
    }

    LEFT_PANEL_ITEMS.scrollTop = prev_scroll;
}

function find_side(entity_id) {
    if (SIDES[SIDE_L].entity_id === entity_id) return SIDE_L;
    if (SIDES[SIDE_R].entity_id === entity_id) return SIDE_R;
    return undefined;
}

function id_to_side(entity_id) {
    if (is_mobile()) return SIDE_L;
    return entity_id.startsWith('conv-') ? SIDE_R : SIDE_L;
}

function close_side(side_i) {
    let side = SIDES[side_i];
    if (side.element) {
        side.element.innerHTML = '';
        side.element.classList.remove('appear');
    }
}

function get_guests(entity_id) {
    let metadata = USER_DATA.entity_map[entity_id];
    let guests = Array.from(metadata.guests);
    guests.push(metadata.author);
    return guests;
}

async function banner_click(event) {
    find('left-panel').classList.remove('hide-mobile');
}

async function init_banner(side_i) {
    let side = SIDES[side_i];

    let banner_c = ['border1-c3-bottom', 'flex-h', 'fs18', 'ai-center', 'bg-lv2'];
    let new_banner = create(null, 'div', banner_c);

    if (side.banner) side.banner.replaceWith(new_banner);
    else side.element.appendChild(new_banner);
    side.banner = new_banner;

    let [c1, c2] = await get_image(side.entity_id);
    let avatar_classes = ['gradient-disk', 'margin05', 'fs08', 'h2'];
    let avatar = create(side.banner, 'div', avatar_classes);
    avatar.style.setProperty('--gc1', c1);
    avatar.style.setProperty('--gc2', c2);

    let name = USER_DATA.secret.entities[side.entity_id].local_name;
    let name_c = ['margin05-right', 'ellipsis'];
    let name_e = create(side.banner, 'b', name_c);
    name_e.innerText = name;

    if (is_mobile()) {
        create(side.banner, 'div', ['grow', 'as-stretch', 'border2-c2-right']);
        let exit_c = ['pad02', 'btn', 'bg-lv3', 'as-stretch', 'square', 'fs15'];
        let exit_btn = create(side.banner, 'button', exit_c);
        exit_btn.addEventListener('click', banner_click);
        exit_btn.innerText = '≡';
    } else {
        const COMMA_C = ['fs08', 'pad03-right'];
        const USERNAME_C = ['fs08', 'link', 'link'];
        const SPAN_C = ['fs08', 'ellipsis'];
        let guests = get_guests(side.entity_id);
        guests = guests.filter(uid => uid !== USER_ID);

        if (guests.length > 0) {
            create(side.banner, 'span', SPAN_C).innerText = '(';

            for (let i = 0; i < guests.length; i++) {
                let username = await get_username(guests[i]);
                let username_btn = create(side.banner, 'span', USERNAME_C);

                username_btn.innerText = username;
                username_btn.addEventListener('click', show_user_card);

                let not_last = (i + 1) < guests.length;
                if (not_last) create(side.banner, 'span', COMMA_C).innerText = ', ';
            }

            create(side.banner, 'span', SPAN_C).innerText = ')';
        } else {
            create(side.banner, 'span', SPAN_C).innerText = '[only you]';
        }
    }
}

async function entity_click() {
    find('left-panel').classList.add('hide-mobile');
    let row = this.parentElement;
    if (row.classList.contains('selected') && !is_mobile()) {
        row.classList.remove('selected');
        let side_i = id_to_side(row.entity_id);
        close_side(side_i);
        SIDES[side_i] = {};
        await refresh_left_panel();
    } else {
        row.classList.add('selected');
        await open_entity(row.entity_id);
    }
}

async function open_entity(entity_id) {
    let side_i = id_to_side(entity_id);
    close_side(side_i);

    let side_id = ['side-l', 'side-r'][side_i];
    let element = find(side_id);
    element.innerHTML = '';

    let [ent_type, raw_id_str] = entity_id.split('-');
    let raw_id = parseInt(raw_id_str);
    SIDES[side_i] = {
        entity_id,
        ent_type,
        element,
        raw_id,
    };

    let init = {
        'conv': init_conv,
        'sheet': console.log,
        'bucket': init_bucket,
        'document': init_document,
    };

    await init[ent_type](side_i);
    setTimeout(() => element.classList.add('appear'), 100);
    await update_last_seen(side_i);
    await refresh_left_panel();
}

async function update_last_seen(side_i) {
    let side = SIDES[side_i];
    USER_DATA.secret.entities[side.entity_id].last_seen_rev = side.revision;
    await request("set-last-seen", [side.entity_id, side.revision]);
}

function show_settings_icon() {
    this.original_text = this.innerText;
    this.innerText = '⚙️';
}

function hide_settings_icon() {
    this.innerText = this.original_text;
}

function type_icon(entity_id) {
    let ent_type = entity_id.split('-')[0];
    if (ent_type === 'conv') return '💬';
    if (ent_type === 'document') return '📄';
    if (ent_type === 'sheet') return '📊';
    if (ent_type === 'bucket') return '🗃️';
    if (ent_type === 'user') return '👤';
    return '[?]';
}

async function send_invite() {
    let parameters = [INVITE_ENTITY_ID, false, [this.user_id]];
    try {
        let _ = await request('create-invite', parameters);
        this.innerText = '✅';
        this.classList.add('disabled');
    } catch (e) {
        alert('Error: ' + e);
    }
}

async function rename_entity() {
    let new_name = prompt('New object name:');
    if (new_name === null) return;

    let parameters = [this.parentElement.entity_id, new_name];
    let [_, user_id] = await request('rename-entity', parameters);

    await load_user_data();
    await refresh_left_panel();
}

async function delete_entity() {
    let entity_id = this.parentElement.entity_id;

    if (!confirm('Delete this object? Other users will keep their access.')) return;

    let is_owner = USER_DATA.entity_map[entity_id].author == USER_ID;
    if (is_owner && !confirm('The first guest will become the owner.')) return;

    let _ = await request('drop', entity_id);

    await load_user_data();
    await refresh_left_panel();

    let side_i = find_side(entity_id);
    if (side_i !== undefined) close_side(side_i);
}

async function show_entity_invite_popup() {
    INVITE_ENTITY_ID = this.parentElement.entity_id;
    await load_user_data();

    let cards = find('user-cards');
    cards.innerHTML = '';

    let guests = get_guests(INVITE_ENTITY_ID);
    let friends = get_friends().filter(uid => !guests.includes(uid));

    if (!friends.length) {
        alert('No friend to invite!');
        return;
    }

    for (let i = 0; i < friends.length; i++) {
        let not_last = (i + 1) != friends.length;
        await add_user_card(friends[i], not_last, true);
    }

    cards.classList.remove('hidden');
    COVER.classList.remove('hidden');
}

async function add_entity(entity_id) {
    let access = USER_DATA.secret.entities[entity_id];
    let current_rev = USER_DATA.entity_map[entity_id].revision;
    
    let out_of_date = access.last_seen_rev !== current_rev;

    let [c1, c2] = await get_image(entity_id);
    let row_classes = ['flex-h', 'jc-center', 'fs12', 'h3', 'noverflow'];
    let side = SIDES[id_to_side(entity_id)];
    if (side.entity_id === entity_id) row_classes.push('selected');
    let row = create(LEFT_PANEL_ITEMS, 'div', row_classes);
    create(LEFT_PANEL_ITEMS, 'div', ['border1-c1-bottom']);

    let container_classes = ['flex-h', 'ai-center', 'grow', 'btn'];
    let cont = create(row, 'div', container_classes);

    let avatar_classes = ['gradient-disk', 'margin05'];
    let avatar = create(cont, 'div', avatar_classes);
    avatar.style.setProperty('--gc1', c1);
    avatar.style.setProperty('--gc2', c2);

    let name_c = ['grow', 'collapse-bye', 'ellipsis', 'left-panel-name'];
    create(cont, 'h4', name_c).innerText = access.local_name;

    let icon_c = ['pad04', 'fs15', 'as-center', 'btn', 'collapse-bye', 'contained'];
    let icon = create(row, 'span', icon_c);
    icon.innerText = (out_of_date ? '◦ ' : '') + type_icon(entity_id);
    icon.addEventListener('mouseenter', show_settings_icon);
    icon.addEventListener('mouseleave', hide_settings_icon);

    let actions = {
        'Rename': rename_entity,
        'Invite': show_entity_invite_popup,
        'Leave': delete_entity,
    };

    init_context_menu(icon, actions);

    cont.addEventListener('click', entity_click);
    row.entity_id = entity_id;
}

async function create_entity(_event, button_text) {
    let text_to_type = {
        'Conversation': 'conv',
        'Document': 'doc',
        'Spreadsheet': 'sheet',
        'Bucket': 'bucket',
    };

    let type = text_to_type[button_text];
    let name = prompt(button_text + ' name:');
    if (!name) return;
    let [_, entity_id] = await request('create-entity', [type, name]);

    await load_user_data();
    await refresh_left_panel();
    COVER.click();
}
