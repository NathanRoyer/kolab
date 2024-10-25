// conv.js

const SRV_AUTHOR = 4294967295;

async function init_conv(side_i) {
    let side = SIDES[side_i];
    let parameters = [side.raw_id, { cursor: 'latest' }];
    let [_, data] = await request('load-messages-before', parameters);
    let [rev, first_msg_index, messages] = data;
    side.revision = rev;

    await init_banner(side_i);

    let msg_c = ['grow', 'border1-c1-bottom', 'flex-v', 'pad1', 'autoverflow', 'ai-stretch'];
    side.msg_div = create(side.element, 'div', msg_c);
    side.msg_div.side_i = side_i;
    side.msg_div.addEventListener('scroll', load_older_messages);
    side.msg_div.first_msg_index = first_msg_index;

    let bottom = create(side.element, 'div', ['pad05', 'flex-h']);

    let input_c = ['grow', 'border1-c2', 'bg-lv2', 'pad05', 'margin05-right', 'radius1']
    let input = create(bottom, 'input', input_c);
    input.type = 'text';
    input.side_i = side_i;
    input.placeholder = 'Write your message here';
    input.addEventListener('input', detect_msg_macro);

    let send_c = ['border2-c1', 'bg-lv2', 'pad05', 'radius1', 'btn'];
    let send = create(bottom, 'button', send_c);
    send.innerText = 'Send';

    redirect_enter(input, send);
    send.addEventListener('click', send_message);

    if (first_msg_index !== 0) {
        let placeholder_c = ['pad05', 'disabled', 'ta-center'];
        let placeholder = create(side.msg_div, 'i', placeholder_c);
        placeholder.innerText = 'Loading older messages...';
    }

    for (let i = 0; i < messages.length; i++) {
        messages[i].index = first_msg_index + i;
        await add_message(side.msg_div, messages[i]);
    }
}

function scrolled_to_max(element) {
    return Math.abs(element.scrollHeight - element.scrollTop - element.clientHeight) < 1;
}

async function add_message(msg_div, message) {
    if (message.author != SRV_AUTHOR) {
        if (message.extended) {
            let special = JSON.parse(message.content);
            message.content = special.content;
            message.reactions = special.reactions;
            message.replying_to = special.replying_to;
            message.edited = special.edited;
        }

        await add_message_ex(msg_div, message);
        let italic_c = ['pad05', 'disabled', 'ta-center'];
    } else {
        create(msg_div, 'i', italic_c).innerText = message.content;
    }
}

async function add_message_ex(msg_div, message) {
    let msg_side = (message.author == USER_ID) ? 'right' : 'left';
    let align_self = (message.author == USER_ID) ? 'as-end' : 'as-start';
    let author_name = await get_username(message.author);

    let scroll_to_bottom = scrolled_to_max(msg_div);

    let authors = msg_div.querySelectorAll('.msg-author');
    let last_author = authors.item(authors.length - 1) || {};
    let last_author_name = last_author.innerText;

    if (last_author_name != author_name && !message.no_author) {
        let author_classes = ['msg-author', align_self, 'link'];
        let author_btn = create(msg_div, 'span', author_classes);
        author_btn.innerText = author_name;
        author_btn.addEventListener('click', show_user_card);
    }

    if (message.replying_to !== undefined) {
        let banner = create(msg_div, 'i', ['link', 'ellipsis', 'align_self']);
        banner.msg_index = message.index;
        banner.innerText = 'Replying to ' + message.replying_to;
    }

    if (message.edited) {
        let align_self = (msg_side === 'right') ? 'as-end' : 'as-start';
        let banner = create(msg_div, 'i', ['fs08', align_self]);
        banner.msg_index = message.index;
        banner.innerText = 'edited on ' + datetime_string(message.edited);
    }

    let row_c = ['msg-' + msg_side, 'flex-h'];
    let row = create(msg_div, 'div', row_c);
    row.msg_index = message.index;

    let btn_c = ['btn', 'pad02', 'fs15', 'as-center', 'radius05', 'w15'];
    let create_btn = (text, callback) => {
        let msg_b = create(row, 'button', btn_c);
        msg_b.innerText = text;
        msg_b.addEventListener('click', callback);
    };

    let add_reactions = async () => {
        if (!message.reactions) return;

        let emojis = Object.keys(message.reactions);
        for (let i = 0; i < emojis.length; i++) {
            let emoji = emojis[i];
            let users = message.reactions[emoji];
            if (!users.length) continue;
            let promises = users.map(get_username);
            let span_c = ['as-center', 'fs12', 'margin05-right'];
            let span = create(row, 'span', span_c);
            span.innerText = emoji;
            span.title = (await Promise.all(promises)).join(', ');
        }
    };

    if (msg_side == 'right' && !is_mobile()) {
        create_btn('+', add_reaction);
        create_btn('ꕯ', edit_message);
        create(row, 'div', ['margin02']);
        await add_reactions();
        create(row, 'div', ['margin02']);
    }

    let msg_c = ['pad05', 'border1-c2', 'bg-lv2', 'wrap-word'];
    let msg_e = create(row, 'span', msg_c);
    msg_e.innerText = message.content;
    msg_e.title = datetime_string(message.created);

    if (msg_side == 'left' && !is_mobile()) {
        create(row, 'div', ['margin02']);
        await add_reactions();
        create_btn('+', add_reaction);
        // create_btn('↩', console.log);
    }

    if (scroll_to_bottom) {
        msg_div.scrollTo(0, msg_div.scrollHeight);
    }
}

async function add_special_message(msg_div, message) {
    let special = JSON.parse(message.content);
    /*__*/ if (special.type == 'metadata') {
        let italic_c = ['pad05', 'disabled', 'ta-center'];
        create(msg_div, 'i', italic_c).innerText = special.message;
    } else if (special.type == 'reply') {
        d
    }
}

function process_token(string) {
    let next = string;
    let output = '';

    while (true) {
        let i = next.indexOf(':');
        if (i == -1) break;

        let remaining = next.substring(i + 1);

        let j = remaining.indexOf(':');
        if (j == -1) break;

        output += next.substring(0, i);
        let token = remaining.substring(0, j);
        let suffix = remaining.substring(j + 1);

        next = suffix;
        let emoji = EMOJI_NAMES[token];
        if (emoji !== undefined) {
            output += emoji;
        } else {
            output += ':' + token + ':';
        }
    }

    return output + next;
}

function detect_msg_macro() {
    this.value = process_token(this.value);
}

async function send_message(event) {
    let input = event.target.parentElement.children[0];
    let side = SIDES[input.side_i];

    if (!input.value.length) return;

    let parameters = [side.raw_id, side.revision, input.value];
    let _ = await request('post-message', parameters);

    input.value = '';
    input.focus();
}

async function load_older_messages(event) {
    let index = this.first_msg_index;
    if (index === 0) return;

    let side = SIDES[this.side_i];
    let loading_placeholder = side.msg_div.children[0];
    let load_more = this.scrollTop < loading_placeholder.clientHeight;
    if (!load_more) return;

    let parameters = [side.raw_id, { cursor: 'specific', index }];
    let [_, data] = await request('load-messages-before', parameters);
    let [_rev, first_msg_index, messages] = data;

    this.first_msg_index = first_msg_index;

    let tmp_msg_div = create(null, 'div', []);
    for (let i = 0; i < messages.length; i++) {
        messages[i].index = first_msg_index + i;
        await add_message(tmp_msg_div, messages[i]);
    }

    let placeholder = side.msg_div.firstChild;
    let msg_elements = Array.from(tmp_msg_div.children);
    let new_node;
    for (let i = 0; i < msg_elements.length; i++) {
        new_node = msg_elements[i];
        new_node.remove();
        side.msg_div.insertBefore(new_node, placeholder);
    }

    new_node.classList.add('border1-c2-bottom');
    placeholder.remove();

    if (first_msg_index !== 0) {
        let first_node = side.msg_div.firstChild;
        side.msg_div.insertBefore(placeholder, first_node);
    }
}

async function add_reaction() {
    let emoji_name = prompt('Discord emoji name:')
    let emoji = EMOJI_NAMES[emoji_name];
    if (!emoji) return;

    let row = this.parentElement;
    let msg_div = row.parentElement;
    let side = SIDES[msg_div.side_i];

    let parameters = [side.raw_id, side.revision, row.msg_index, emoji];
    await request('toggle-reaction', parameters);
}

async function edit_message() {
    let new_content = prompt('New content:')
    if (!new_content) return;

    let row = this.parentElement;
    let msg_div = row.parentElement;
    let side = SIDES[msg_div.side_i];

    let parameters = [side.raw_id, side.revision, row.msg_index, new_content];
    await request('edit-message', parameters);
}

async function set_message(side_i, message) {
    let side = SIDES[side_i];

    let tmp_msg_div = create(null, 'div', []);
    message.no_author = true;
    await add_message(tmp_msg_div, message);
    delete message.no_author;

    let ref_node;
    let nodes = Array.from(side.msg_div.children);
    for (let i = 0; i < nodes.length; i++) {
        let node = nodes[i];
        if (node.msg_index == message.index) {
            ref_node = node.nextSibling;
            node.remove();
        } else if (ref_node) {
            break;
        }
    }

    console.log(ref_node);

    let children = Array.from(tmp_msg_div.children);
    for (let i = 0; i < children.length; i++) {
        let node = children[i];
        node.remove();
        if (ref_node) side.msg_div.insertBefore(node, ref_node);
        else side.msg_div.appendChild(node);
    }
}
