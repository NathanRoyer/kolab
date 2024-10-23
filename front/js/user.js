// account.js

async function shutdown_server() {
    if (confirm('Save a database backup and shutdown the server?')) {
        let _ = await request('server-shutdown', null);
    }
}

function refresh_invites_button() {
    let list_invites = find('list-invites');
    if (USER_DATA.secret.invites.length) {
        list_invites.classList.add('selected');
    } else {
        list_invites.classList.remove('selected');
    }
}

async function load_user_data() {
    let [_, parameters] = await request('load-user-data', null);
    let [revision, public, entity_map, secret] = parameters;

    USER_DATA = {
        revision,
        public,
        entity_map,
        secret,
    };

    refresh_invites_button();

    let user_actions = {
        'Friends': show_friends,
        'Find User': find_user,
        'Set Status': user_click,
        'Theme': {
            'Dark': switch_theme,
            'Elodie': switch_theme,
        },
        'Log Out': log_out,
    };

    if (USER_DATA.secret.server_admin) {
        user_actions['Shutdown Server'] = shutdown_server;
    }

    init_context_menu(find('settings-btn'), user_actions);
}

function set_user_tile(username, status, c1, c2) {
    let avatar = find('user-avatar');
    avatar.style.setProperty('--gc1', c1);
    avatar.style.setProperty('--gc2', c2);
    find('user-name').innerText = username;
    find('user-status').innerText = status;
}

async function login_auth() {
    let token = localStorage['token-' + USER_ID];
    let _success = await request('open-session', [USER_ID, token]);

    await load_user_data();
    await refresh_left_panel();

    CAN_CLOSE_POPUP = true;
    COVER.click();
    find('password-input').value = '';
}

async function login_token() {
    let password = find('password-input').value;

    try {
        let parameters = [USER_ID, password];
        let [_, token] = await request('get-token', parameters);
        localStorage['user-id'] = USER_ID;
        localStorage['token-' + USER_ID] = token;
    } catch {
        alert("Invalid Password!");
        return;
    }

    await login_auth();
}

async function check_username() {
    try {
        let username = find('username-input').value;
        let [_, user_id] = await request('who-is', username);
        USER_ID = user_id;
        await login_token();
    } catch (e) {
        alert(e);
    }
}

async function create_account() {
    let username = find('username-input').value;
    let password = find('password-input').value;

    if (username.length < 3) {
        alert('Username must be at least 3 characters long');
        return;
    }

    let username_regex = /^[a-z]+([a-z0-9]|\-)*[a-z0-9]+/;
    if (!username_regex.test(username)) {
        alert('Invalid username');
        return;
    }

    if (password.length < 4) {
        alert('Password must be at least 4 characters long');
        return;
    }

    try {
        let parameters = [username, password];
        let [_, user_id] = await request('create-account', parameters);
        USER_ID = user_id;
        localStorage['user-id'] = user_id;
    } catch {
        alert("Username already taken!");
        return;
    }

    await login_token();
}

async function login_or_create_account() {
    let login = find('login-button').classList.contains('selected');
    await login ? check_username() : create_account();
}

async function send_friend_request() {
    let parameters = ['user-' + USER_ID, true, [this.user_id]];
    let _ = await request('create-invite', parameters);
    alert('Friend request sent');
}

async function open_invite() {
    let index = this.parentElement.invite_index;
    let parameters = [USER_DATA.revision, index, this.discard];
    let _ = await request('open-invite', parameters);
    
    await load_user_data();
    await refresh_left_panel();
    await show_invites_list();
}

async function show_invites_list() {
    find('list-invites').classList.remove('selected');
    let invites_list = find('invites-list');
    invites_list.innerHTML = '';

    await load_user_data();

    if (!USER_DATA.secret.invites.length) {
        let row = create(invites_list, 'span', ['margin05']);
        row.innerText = '(Nothing to show)';
    }

    for (let i = 0; i < USER_DATA.secret.invites.length; i++) {
        let invite = USER_DATA.secret.invites[i];

        let row = create(invites_list, 'div', ['flex-h', 'margin05', 'border1-c2']);
        row.invite_index = i;
        row.target = invite.target;
        let sender_username = await get_username(invite.sender);

        let span_classes = ['pad05', 'border1-c2-right'];
        let name_classes = ['pad05', 'border1-c2-right', 'grow'];
        create(row, 'span', span_classes).innerText = type_icon(invite.target);
        create(row, 'span', name_classes).innerText = invite.orig_name;
        create(row, 'span', span_classes).innerText = 'from: ' + sender_username;
        // create(row, 'span', span_classes).innerText = invite.read_only ? 'RO' : 'RW';

        let accept_btn = create(row, 'button', ['pad05', 'border1-c2-right', 'btn']);
        let reject_btn = create(row, 'button', ['pad05', 'btn']);

        accept_btn.innerText = '✅';
        accept_btn.discard = false;
        reject_btn.innerText = '❎';
        reject_btn.discard = true;
        accept_btn.addEventListener('click', open_invite);
        reject_btn.addEventListener('click', open_invite);
    }

    find('invites-popup').classList.remove('hidden');
    COVER.classList.remove('hidden');
}

async function user_click() {
    let user_status = find('user-status');
    let new_status = prompt('New Status:', user_status.innerText);
    if (new_status) {
        USER_DATA.public.status = new_status;
        let parameters = [USER_DATA.revision, USER_DATA.public];
        let _ = await request('set-user-data', parameters);
        await refresh_left_panel();
    }
}

async function log_out() {
    delete localStorage['user-id'];
    location.reload();
}

function find_conversation(user_id) {
    console.log(user_id);
    let entity_ids = Object.keys(USER_DATA.secret.entities);
    for (let i = 0; i < entity_ids.length; i++) {
        let entity_id = entity_ids[i];
        if (!entity_id.startsWith('conv-')) continue;
        let guests = get_guests(entity_id);
        console.log('guests:', guests);

        let has_us = guests.indexOf(USER_ID) >= 0;
        let has_friend = guests.indexOf(user_id) >= 0;
        let two_guests = guests.length == 2;
        if (two_guests && has_us && has_friend) return entity_id;
    }

    return null;
}

async function open_friend_conv() {
    if (is_friend(this.user_id)) {
        let conv_id = find_conversation(this.user_id);

        if (conv_id !== null) {

            await open_entity(conv_id);
            find('user-cards').classList.add('hidden');
            COVER.click();

        } else if (confirm('Create conversation with this user?')) {
            let username = await get_username(this.user_id);
            let param_conv = ['conv', username];
            let [_a, entity_id] = await request('create-entity', param_conv);

            await load_user_data();
            await refresh_left_panel();

            let param_invite = [entity_id, false, [this.user_id]];
            let _b = await request('create-invite', param_invite);
        }

    } else if (confirm('Send friend request?')) {
        await send_friend_request.call({ user_id: this.user_id });
    }
}

async function add_user_card(user_id, not_last, invite) {
    let [_b, parameters] = await request('load-user-data', user_id);
    let [rev, public, image] = parameters;

    let card_classes = ['flex-h', 'grow', 'h4'];
    if (not_last) card_classes.push('border2-c1-bottom');
    let card = create(find('user-cards'), 'div', card_classes);

    let [c1, c2] = image.data;
    let avatar_classes = ['gradient-disk', 'margin1', 'fs08'];
    let avatar = create(card, 'div', avatar_classes);
    avatar.style.setProperty('--gc1', c1);
    avatar.style.setProperty('--gc2', c2);

    let texts = create(card, 'div', ['flex-v', 'jc-center', 'grow']);
    create(texts, 'h3', []).innerText = public.name;
    create(texts, 'span', []).innerText = public.status;

    let conv_c = ['flex-v', 'jc-center', 'pad05', 'fs15', 'btn', 'square', 'ta-center'];
    let conv_btn = create(card, 'div', conv_c);
    conv_btn.user_id = user_id;

    if (user_id == USER_ID) conv_btn.classList.add('disabled');

    if (invite) {
        conv_btn.innerText = '📤';
        conv_btn.addEventListener('click', send_invite);
    } else {
        conv_btn.innerText = is_friend(user_id) ? '💬' : '👤';
        conv_btn.addEventListener('click', open_friend_conv);
    }
}

async function show_user_card() {
    let username = this.innerText;
    let [_a, user_id] = await request('who-is', username);
    let cards = find('user-cards');
    cards.innerHTML = '';

    await add_user_card(user_id, false, false);

    cards.classList.remove('hidden');
    COVER.classList.remove('hidden');
}

async function find_user() {
    let username = prompt('Enter a username:');
    if (!username) return;

    try {
        let self = { innerText: username };
        await show_user_card.call(self);
    } catch (e) {
        alert('Error: ' + e);
    }
}

function is_friend(user_id) {
    return USER_DATA.secret.entities['user-' + user_id] !== undefined;
}

function get_friends() {
    let friends = [];

    let entity_ids = Object.keys(USER_DATA.secret.entities);
    for (let i = entity_ids.length - 1; i >= 0; i--) {
        let entity_id = entity_ids[i];
        if (!entity_id.startsWith('user-')) continue;
        let user_id = parseInt(entity_id.substring(5));
        friends.push(user_id);
    }

    return friends;
}

async function show_friends() {
    let cards = find('user-cards');
    cards.innerHTML = '';

    await load_user_data();
    let friends = get_friends();

    if (!friends.length) {
        let redirect = confirm('No friends yet! Do you want to find a specific user?');
        if (redirect) await find_user();
        return;
    }

    for (let i = 0; i < friends.length; i++) {
        let not_last = (i + 1) != friends.length;
        await add_user_card(friends[i], not_last, false);
    }

    cards.classList.remove('hidden');
    COVER.classList.remove('hidden');
}

function switch_theme(_event, theme) {
    let body_id;

    /**/ if (theme == 'Dark') body_id = 'body-theme-dark';
    else if (theme == 'Elodie') body_id = 'body-theme-elodie';

    localStorage['theme'] = body_id;
    document.body.id = body_id;
}
