// conv.js

async function init_conv(side_i) {
    let side = SIDES[side_i];
    let parameters = [side.raw_id, { cursor: 'latest' }];
    let [_, data] = await request('load-messages-before', parameters);
    let [rev, first_msg_index, messages] = data;
    side.revision = rev;

    await init_banner(side_i);

    let msg_c = ['grow', 'border1-c1-bottom', 'flex-v', 'pad1', 'autoverflow'];
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
        await add_message(side.msg_div, messages[i]);
    }
}

function scrolled_to_max(element) {
    return Math.abs(element.scrollHeight - element.scrollTop - element.clientHeight) < 1;
}

async function add_message(msg_div, message) {
    let msg_side = (message.author == USER_ID) ? 'right' : 'left';
    let author_name = await get_username(message.author);

    let scroll_to_bottom = scrolled_to_max(msg_div);

    let authors = msg_div.querySelectorAll('.msg-author');
    let last_author = authors.item(authors.length - 1) || {};
    let last_author_name = last_author.innerText;

    if (last_author_name != author_name) {
        let author_classes = ['msg-author', 'msg-' + msg_side, 'link'];
        let author_btn = create(msg_div, 'span', author_classes);
        author_btn.innerText = author_name;
        author_btn.addEventListener('click', show_user_card);
    }

    let message_classes = ['pad05', 'border1-c2', 'bg-lv2', 'wrap-word', 'msg-' + msg_side];
    let msg_e = create(msg_div, 'span', message_classes);
    msg_e.innerText = message.content;
    msg_e.title = datetime_string(message.created);

    if (scroll_to_bottom) {
        msg_div.scrollTo(0, msg_div.scrollHeight);
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
        await add_message(tmp_msg_div, messages[i]);
    }

    let placeholder = side.msg_div.firstChild;
    let msg_elements = tmp_msg_div.children;
    for (let i = 0; i < msg_elements.length; i++) {
        let new_node = msg_elements[i];
        side.msg_div.insertBefore(new_node, placeholder);
    }

    placeholder.remove();

    if (first_msg_index !== 0) {
        let first_node = side.msg_div.firstChild;
        side.msg_div.insertBefore(placeholder, first_node);
    }
}
