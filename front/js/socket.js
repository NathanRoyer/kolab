// socket.js

async function reply_callback(event) {
    let reply_obj = JSON.parse(event.data);

    // is this an update?
    if (reply_obj.num === undefined) {
        await handle_update(reply_obj);
        return;
    }

    let resolve, reject;
    for (let i = 0; i < CALLBACKS.length; i++) {
        if (CALLBACKS[i].num == reply_obj.num) {
            resolve = CALLBACKS[i].resolve;
            reject = CALLBACKS[i].reject;
            CALLBACKS.splice(i, 1);
        }
    }

    if (resolve === undefined) {
        console.error('Failed to match reply with a request');
        return;
    }

    if (reply_obj.reply == 'generic-failure') {
        reject(reply_obj.parameters);
    } else {
        resolve([reply_obj.reply, reply_obj.parameters]);
    }
}

async function handle_update(update) {
    console.log(update.type, update.id);
    let notif_enabled = Notification.permission === 'granted';

    if (update.id === 'user-' + USER_ID) {
        if (update.type === 'new-invite') {
            if (notif_enabled) {
                let sender = await get_username(update.data.sender);
                let text = update.data.orig_name +  '(' + sender + ')';
                new Notification('New Invite', { body: text });
            }

            if (!find('invites-popup').classList.contains('hidden')) {
                find('list-invites').click();
            }

            await load_user_data();
            refresh_invites_button();
        } else if (update.type === 'new-friend') {
            if (notif_enabled) {
                await load_user_data();
                let username = await get_username(update.index);
                let text = username + ' accepted your friend request!';
                new Notification('New Friend', { body: text });
            }
        } else if (update.type === 'set-user') {
            if (update.id == 'user-' + USER_ID) {
                await load_user_data();
                await refresh_left_panel();
            }
        }

        return;
    }

    // can be undefined
    let side_i = find_side(update.id);
    let side = SIDES[side_i];

    /*__*/ if (update.type === 'new-message') {
        let message = update.data;

        if (!side && notif_enabled && message.author != USER_ID) {
            let text = await get_username(message.author) + ': ' + message.content;
            let title = USER_DATA.secret.entities[update.id].local_name;
            new Notification(title, { body: text });
        }

        if (side) {
            side.revision = update.new_revision;
            await add_message(side.msg_div, message);
            await update_last_seen(side_i);
        }
    } else if (update.type === 'new-guest' || update.type === 'bye-guest') {
        await load_user_data();
        await refresh_left_panel();
        if (side) await init_banner(side_i);

        if (notif_enabled) {
            let suffix = (update.type === 'new-guest') ? ' joined!' : ' left';
            let text = await get_username(update.data) + suffix;
            let title = USER_DATA.secret.entities[update.id].local_name;
            new Notification(title, { body: text });
        }
    } else if (update.type === 'bye-element' && side) {
        side.elem_div.children[update.index].remove();
        side.elements.splice(update.index, 1);
        side.revision = update.new_revision;
        await update_last_seen(side_i);
    } else if (update.type === 'set-element' && side) {
        let node = element_node(update.data);
        side.elem_div.children[update.index].replaceWith(node);
        side.elements[update.index] = update.data;
        side.revision = update.new_revision;
        await update_last_seen(side_i);
    } else if (update.type === 'new-element' && side) {
        let new_node = element_node(update.data);
        side.revision = update.new_revision;

        if (side.childElementCount == update.index) {
            side.elem_div.appendChild(new_node);
            side.elements.push(update.data);
        } else {
            let ref_node = side.elem_div.children[update.index];
            side.elem_div.insertBefore(new_node, ref_node);
            side.elements.splice(update.index, 0, update.data);
        }
        await update_last_seen(side_i);
    } else if (update.type.endsWith('-file') && side) {
        await open_entity(update.id);
    }

    if (side === undefined) {
        await load_user_data();
        await refresh_left_panel();
    }
}

function request(request, parameters) {
    let num = NEXT_REQ_NUM;
    NEXT_REQ_NUM += 1;

    let setup = (resolve, reject) => {
        CALLBACKS.push({ num, resolve, reject });
        SOCKET.send(JSON.stringify({ num, request, parameters }));
    };

    return new Promise(setup);
}

function ws_disconnected() {
    if (confirm("Connection failure! Reload page?")) location.reload();
}

function init_websocket() {
    let protocol = document.location.protocol === 'http:' ? 'ws:' : 'wss:';
    let ws_url = protocol + '//' + document.location.host + '/session';

    SOCKET = new WebSocket(ws_url);
    SOCKET.addEventListener('open', try_auto_login);
    SOCKET.addEventListener('message', reply_callback);
    SOCKET.addEventListener('close', ws_disconnected);
    SOCKET.addEventListener('error', ws_disconnected);
}
