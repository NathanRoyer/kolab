// doc.js

const DOC_ACTIONS = {
    'Title': push_element,
    'Part': push_element,
    'Chapter': push_element,
    'Section': push_element,
    'Subsection': push_element,
    'Image': push_element,
    'Paragraph': push_element,
};

async function save_paragraph_changes() {
    await change_element(this, this.value);
}

async function edit_paragraph() {
    let textarea_c = ['bg-lv1', 'border1-c2', 'radius05', 'pad05'];
    let textarea = create(null, 'textarea', textarea_c);
    let height = 'calc(' + this.clientHeight + 'px - 1em)';
    textarea.style.setProperty('height', height);
    textarea.value = this.innerText;
    textarea.addEventListener('focusout', save_paragraph_changes);
    this.replaceWith(textarea);
    textarea.focus();
}

async function edit_text_based() {
    let new_text = prompt('New text:', this.innerText);
    if (new_text) await change_element(this, new_text);
}

async function edit_image() {
    let new_src = prompt('New image link:', this.src);
    if (new_src) await change_element(this, new_src);
}

async function change_element(node, data) {
    let side = SIDES[node.parentElement.side_i];
    let index = child_index(node);
    let element = side.elements[index];
    element.data = data;

    let parameters = [side.raw_id, side.revision, index, element];
    let _ = await request('set-element', parameters);
}

async function delete_element() {
    let side = SIDES[this.parentElement.side_i];
    let index = child_index(this);
    let parameters = [side.raw_id, side.revision, index];
    let _ = await request('delete-element', parameters);
}

const DEFAULT_IMG_URL = location.origin + '/files/background.png';

async function push_element(_event, style_name) {
    let style = style_name.toLowerCase();
    let data = (style == 'image') ? DEFAULT_IMG_URL : 'Sample Text';

    let element = {
        'data': data,
        'style': style,
    };

    let index, side;
    if (this.side_i === undefined) {
        side = SIDES[this.parentElement.side_i];
        let must_add_one = CONTEXT_MENU.user_path[0] === 'Insert After';
        index = child_index(this) + (must_add_one ? 1 : 0);
    } else {
        side = SIDES[this.side_i];
        index = this.childElementCount;
    }

    let parameters = [side.raw_id, side.revision, index, element];
    let _ = await request('insert-element', parameters);
}

function element_node(element) {
    let elem_c = ['btn', 'pad10px', 'radius05'];
    let editor = edit_text_based;
    let style = element.style;
    let data = element.data;

    let node;
    /*__*/ if (style === 'title') {
        node = create(null, 'h1', elem_c)
        node.innerText = data;
    } else if (style === 'part') {
        node = create(null, 'h2', elem_c)
        node.innerText = data;
    } else if (style === 'chapter') {
        node = create(null, 'h3', elem_c)
        node.innerText = data;
    } else if (style === 'section') {
        node = create(null, 'h4', elem_c)
        node.innerText = data;
    } else if (style === 'subsection') {
        node = create(null, 'h5', elem_c)
        node.innerText = data;
    } else if (style === 'image') {
        let img_c = elem_c.concat(['doc-image', 'bg-lv1', 'radius05']);
        node = create(null, 'img', img_c)
        node.src = data;
        editor = edit_image;
    } else if (style === 'paragraph') {
        node = create(null, 'p', elem_c);
        node.innerText = data;
        editor = edit_paragraph;
    }

    let action_map = {
        'Insert Before': DOC_ACTIONS,
        'Insert After': DOC_ACTIONS,
        'Edit': editor,
        'Delete': delete_element,
    };

    init_context_menu(node, action_map);
    return node;
}

async function init_document(side_i) {
    let side = SIDES[side_i];
    let [_, [rev, elements]] = await request('load-document', side.raw_id);
    side.revision = rev;
    side.elements = elements;

    await init_banner(side_i);

    let restore_scroll = null;
    if (side.elem_div) {
        restore_scroll = side.elem_div.scrollTop;
        side.elem_div.remove();
    }

    let div_c = ['grow', 'flex-v', 'pad05', 'autoverflow'];
    side.elem_div = create(side.element, 'div', div_c);
    side.elem_div.side_i = side_i;

    init_context_menu(side.elem_div, DOC_ACTIONS);

    for (let i = 0; i < elements.length; i++) {
        let element = side.elements[i];
        let node = element_node(element);
        side.elem_div.appendChild(node);
    }

    if (restore_scroll !== null) {
        side.elem_div.scrollTo(0, restore_scroll);
    }
}
