let tasks = [];
let currentTab = 'all';
let recognition = null;
let editingTaskId = null;
let isAdding = false;
let isAlwaysOnTop = false;
let searchVisible = false;
let isCollapsed = false;

const STATUS_MAP = {
  '待办': { label: '待办', class: 'tag-pending', code: 'pending' },
  '进行中': { label: '进行中', class: 'tag-progress', code: 'progress' },
  '已完成': { label: '已批', class: 'tag-done', code: 'done' }
};

async function invoke(cmd, args = {}) {
  console.log('invoke: ' + cmd);
  try {
    if (!window.__TAURI__) {
      throw new Error('window.__TAURI__ is undefined');
    }
    if (!window.__TAURI__.core) {
      throw new Error('window.__TAURI__.core is undefined');
    }
    return await window.__TAURI__.core.invoke(cmd, args);
  } catch (error) {
    console.log('invoke error ' + cmd + ': ' + error.message);
    showToast('调用失败: ' + error.message);
    throw error;
  }
}

function initVoice() {
  if ('webkitSpeechRecognition' in window || 'SpeechRecognition' in window) {
    const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
    recognition = new SpeechRecognition();
    recognition.lang = 'zh-CN';
    recognition.continuous = false;
    recognition.interimResults = false;
    recognition.onstart = () => showToast('正在聆听...');
    recognition.onend = () => {};
    recognition.onresult = (event) => {
      const transcript = event.results[0][0].transcript;
      if (transcript.toLowerCase().includes('todolist')) {
        const taskText = transcript.replace(/todolist/i, '').trim();
        if (taskText) addTask(taskText);
      } else {
        showToast('请说 "TODOList" 开头');
      }
    };
    recognition.onerror = () => showToast('语音识别失败');
  }
}

function extractDeadline(text) {
  const now = new Date();
  const year = now.getFullYear();
  const month = now.getMonth() + 1;
  const day = now.getDate();
  if (text.includes('今天')) return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')} 00:00:00`;
  if (text.includes('明天')) {
    const t = new Date(now); t.setDate(t.getDate() + 1);
    return `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, '0')}-${String(t.getDate()).padStart(2, '0')} 00:00:00`;
  }
  if (text.includes('后天')) {
    const t = new Date(now); t.setDate(t.getDate() + 2);
    return `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, '0')}-${String(t.getDate()).padStart(2, '0')} 00:00:00`;
  }
  if (text.includes('下周')) {
    const t = new Date(now); t.setDate(t.getDate() + 7);
    return `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, '0')}-${String(t.getDate()).padStart(2, '0')} 00:00:00`;
  }
  if (text.includes('下月') || text.includes('下个月')) {
    const t = new Date(now); t.setMonth(t.getMonth() + 1);
    return `${t.getFullYear()}-${String(t.getMonth() + 1).padStart(2, '0')}-${String(t.getDate()).padStart(2, '0')} 00:00:00`;
  }
  return null;
}

function cleanTaskName(text) {
  return text.replace(/(今天|明天|后天|下周|下月|下个月)\s*/g, '').trim();
}

async function loadTasks() {
  showLoading();
  try {
    const data = await invoke('get_tasks');
    if (data.code === 0 && data.data) {
      tasks = data.data;
      console.log('loaded ' + tasks.length + ' tasks');
    } else {
      tasks = [];
      console.log('load failed: ' + (data.msg || 'unknown'));
    }
    renderTasks();
  } catch (error) {
    console.error('Load error:', error);
    showToast('无法连接到后端');
  } finally {
    hideLoading();
  }
}

async function addTask(name, priority = '中', deadline = null) {
  const autoDeadline = extractDeadline(name);
  const cleanName = cleanTaskName(name);
  const finalDeadline = deadline ? `${deadline} 00:00:00` : autoDeadline;
  showLoading();
  try {
    const data = await invoke('create_task', { name: cleanName, deadline: finalDeadline, priority });
    if (data.code === 0) {
      showToast('已添加');
      await loadTasks();
    } else {
      showToast(data.msg || '添加失败');
    }
  } catch (error) {
    showToast('添加失败');
  } finally {
    hideLoading();
  }
}

async function updateTask(id, name, priority, deadline, link, note) {
  showLoading();
  try {
    const payload = { 'id': id, '任务名称': name };
    if (priority) payload['优先级'] = priority;
    if (deadline) payload['截止时间'] = deadline + ' 00:00:00';
    if (link !== undefined) payload['链接'] = link;
    if (note !== undefined) payload['备注'] = note;
    const data = await invoke('update_task', { payload });
    if (data.code === 0) {
      showToast('已更新');
      await loadTasks();
    } else {
      showToast(data.msg || '更新失败');
    }
  } catch (error) {
    showToast('更新失败');
  } finally {
    hideLoading();
  }
}

async function toggleTaskStatus(id) {
  const task = tasks.find(t => t.id === id);
  if (!task) return;
  const newStatus = task.status === '已完成' ? '待办' : '已完成';
  const completedAt = newStatus === '已完成' ? formatDateTimeLocal(new Date()) : null;
  showLoading();
  try {
    const payload = { 'id': id, '状态': newStatus };
    if (completedAt) payload['完成时间'] = completedAt;
    else payload['完成时间'] = null;
    const data = await invoke('update_task', { payload });
    if (data.code === 0) {
      showToast(newStatus === '已完成' ? '已批' : '已恢复');
      await loadTasks();
    } else {
      showToast(data.msg || '更新失败');
    }
  } catch (error) {
    showToast('更新失败');
  } finally {
    hideLoading();
  }
}

async function deleteTask(id) {
  showLoading();
  try {
    const data = await invoke('delete_task', { id });
    if (data.code === 0) {
      showToast('已删除');
      await loadTasks();
    } else {
      showToast(data.msg || '删除失败');
    }
  } catch (error) {
    showToast('删除失败');
  } finally {
    hideLoading();
  }
}

function isToday(dateStr) {
  if (!dateStr) return false;
  const date = new Date(dateStr);
  const today = new Date();
  return date.toDateString() === today.toDateString();
}

function isFuture(dateStr) {
  if (!dateStr) return false;
  const date = new Date(dateStr);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  date.setHours(0, 0, 0, 0);
  return date > today;
}

function isOverdue(dateStr) {
  if (!dateStr) return false;
  const date = new Date(dateStr);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  date.setHours(0, 0, 0, 0);
  return date < today;
}

function sortByPriority(a, b) {
  const priorityOrder = { '高': 0, '中': 1, '低': 2 };
  const aPriority = priorityOrder[(a.priority || '').trim()] ?? 2;
  const bPriority = priorityOrder[(b.priority || '').trim()] ?? 2;
  if (aPriority !== bPriority) return aPriority - bPriority;
  if (a.deadline && b.deadline) return new Date(a.deadline) - new Date(b.deadline);
  return 0;
}

function getTaskTag(task) {
  if (task.status === '已完成') return { label: '已批', class: 'tag-done' };
  if (!task.deadline) return { label: '随时', class: 'tag-anytime' };
  if (isToday(task.deadline)) return { label: '今日', class: 'tag-today' };
  if (isOverdue(task.deadline)) return { label: '逾期', class: 'tag-overdue' };
  return { label: '计划', class: 'tag-planned' };
}

function getPriorityClass(priority) {
  switch (priority) {
    case '高': return 'tag-overdue';
    case '中': return 'tag-planned';
    case '低': return 'tag-anytime';
    default: return 'tag-planned';
  }
}

function formatDate(dateStr) {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  const today = new Date();
  const tomorrow = new Date(today);
  tomorrow.setDate(tomorrow.getDate() + 1);
  if (date.toDateString() === today.toDateString()) return '';
  if (date.toDateString() === tomorrow.toDateString()) return '明日';
  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' });
}

function formatDateTimeLocal(date) {
  const pad = (n) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function formatDateInput(date) {
  const pad = (n) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function getFilterValue(id) {
  const el = document.getElementById(id);
  return el ? el.value.trim() : '';
}

function hasActiveFilters() {
  return !!(getFilterValue('searchKeyword') || getFilterValue('searchPriority') ||
    getFilterValue('searchCreatedFrom') || getFilterValue('searchCreatedTo') ||
    getFilterValue('searchCompletedFrom') || getFilterValue('searchCompletedTo'));
}

function filterTasks(list) {
  if (!hasActiveFilters()) return list;
  const keyword = getFilterValue('searchKeyword').toLowerCase();
  const priority = getFilterValue('searchPriority');
  const createdFrom = getFilterValue('searchCreatedFrom');
  const createdTo = getFilterValue('searchCreatedTo');
  const completedFrom = getFilterValue('searchCompletedFrom');
  const completedTo = getFilterValue('searchCompletedTo');
  return list.filter(task => {
    if (keyword) {
      const text = `${task.name} ${task.note || ''} ${task.link || ''}`.toLowerCase();
      if (!text.includes(keyword)) return false;
    }
    if (priority && task.priority !== priority) return false;
    if (createdFrom || createdTo) {
      if (!task.created_at) return false;
      const d = formatDateInput(new Date(task.created_at));
      if (createdFrom && d < createdFrom) return false;
      if (createdTo && d > createdTo) return false;
    }
    if (completedFrom || completedTo) {
      if (!task.completed_at) return false;
      const d = formatDateInput(new Date(task.completed_at));
      if (completedFrom && d < completedFrom) return false;
      if (completedTo && d > completedTo) return false;
    }
    return true;
  });
}

function toggleSearch() {
  searchVisible = !searchVisible;
  document.getElementById('searchPanel').classList.toggle('show', searchVisible);
  document.getElementById('searchBtn').classList.toggle('active', searchVisible);
}

function applyFilters() { renderTasks(); }

function clearFilters() {
  document.getElementById('searchKeyword').value = '';
  document.getElementById('searchPriority').value = '';
  document.getElementById('searchCreatedFrom').value = '';
  document.getElementById('searchCreatedTo').value = '';
  document.getElementById('searchCompletedFrom').value = '';
  document.getElementById('searchCompletedTo').value = '';
  renderTasks();
}

function renderAddTask() {
  return `
    <div class="task-item is-adding">
      <div class="checkbox"></div>
      <div class="task-content">
        <textarea class="edit-input" id="addInput" rows="2" placeholder="要做什么..." onkeydown="handleAddKey(event)"></textarea>
        <div class="edit-actions">
          <select class="edit-input" id="addPriority" style="width: auto; min-height: auto; padding: 4px 8px; font-size: 13px; border-color: var(--border);">
            <option value="高">高</option>
            <option value="中" selected>中</option>
            <option value="低">低</option>
          </select>
          <input type="date" class="edit-input" id="addDeadline" value="" style="width: auto; min-height: auto; padding: 4px 8px; font-size: 13px; border-color: var(--border);" />
        </div>
        <div class="edit-actions">
          <button class="save-btn" onclick="saveAdd()">添加</button>
          <button onclick="cancelAdd()">取消</button>
        </div>
      </div>
    </div>
  `;
}

function renderTask(task) {
  const isDone = task.status === '已完成';
  const tag = getTaskTag(task);
  const dateLabel = task.deadline ? formatDate(task.deadline) : '';
  if (editingTaskId === task.id) {
    return `
      <div class="task-item">
        <div class="checkbox ${isDone ? 'checked' : ''}"><svg viewBox="0 0 24 24"><polyline points="20 6 9 17 4 12"/></svg></div>
        <div class="task-content">
          <textarea class="edit-input" id="editInput-${task.id}" rows="2" onkeydown="handleEditKey(event, '${task.id}')">${escapeHtml(task.name)}</textarea>
          <div class="edit-actions">
            <select class="edit-input" id="editPriority-${task.id}" style="width: auto; min-height: auto; padding: 4px 8px; font-size: 13px; border-color: var(--border);">
              <option value="高" ${task.priority === '高' ? 'selected' : ''}>高</option>
              <option value="中" ${task.priority === '中' ? 'selected' : ''}>中</option>
              <option value="低" ${task.priority === '低' ? 'selected' : ''}>低</option>
            </select>
            <input type="date" class="edit-input" id="editDeadline-${task.id}" value="${task.deadline ? task.deadline.split(' ')[0] : ''}" style="width: auto; min-height: auto; padding: 4px 8px; font-size: 13px; border-color: var(--border);" />
          </div>
          <div class="edit-actions">
            <input type="text" class="edit-input" id="editLink-${task.id}" value="${escapeHtml(task.link || '')}" placeholder="链接 URL" style="flex: 1; min-height: auto; padding: 4px 8px; font-size: 13px; border-color: var(--border);" />
          </div>
          <div class="edit-actions">
            <textarea class="edit-input" id="editNote-${task.id}" rows="2" placeholder="备注..." style="flex: 1; min-height: auto; padding: 4px 8px; font-size: 13px; border-color: var(--border); resize: vertical;">${escapeHtml(task.note || '')}</textarea>
          </div>
          <div class="edit-actions">
            <button class="save-btn" onclick="saveEdit('${task.id}')">保存</button>
            <button onclick="cancelEdit()">取消</button>
          </div>
        </div>
      </div>
    `;
  }
  return `
    <div class="task-item ${isDone ? 'completed' : ''}">
      <div class="checkbox ${isDone ? 'checked' : ''}" onclick="event.stopPropagation(); toggleTaskStatus('${task.id}')">
        <svg viewBox="0 0 24 24"><polyline points="20 6 9 17 4 12"/></svg>
      </div>
      <div class="task-content">
        <div class="task-title">${escapeHtml(task.name)}</div>
        <div class="task-meta">
          ${task.priority !== '中' ? `<span class="task-tag ${getPriorityClass(task.priority)}">${task.priority}</span>` : ''}
          ${tag ? `<span class="task-tag ${tag.class}">${tag.label}</span>` : ''}
          ${dateLabel ? `<span class="task-date">${dateLabel}</span>` : ''}
        </div>
        ${task.note ? `<div class="task-note">${escapeHtml(task.note)}</div>` : ''}
        ${task.link ? `<a class="task-link" href="#" data-url="${escapeHtml(normalizeUrl(task.link))}">${escapeHtml(task.link)}</a>` : ''}
      </div>
      <div class="task-actions">
        <button class="edit-btn" onclick="event.stopPropagation(); startEdit('${task.id}')" title="编辑">改</button>
        <button class="edit-btn delete-btn" onclick="event.stopPropagation(); deleteTask('${task.id}')" title="删除">删</button>
      </div>
    </div>
  `;
}

function renderTasks() {
  const isTaskDone = (t) => t.status === '已完成';
  const allRaw = tasks.filter(t => !isTaskDone(t)).sort(sortByPriority);
  const todayRaw = tasks.filter(t => !isTaskDone(t) && isToday(t.deadline)).sort(sortByPriority);
  const scheduledRaw = tasks.filter(t => !isTaskDone(t) && t.deadline && !isToday(t.deadline)).sort(sortByPriority);
  const anytimeRaw = tasks.filter(t => !isTaskDone(t) && !t.deadline).sort(sortByPriority);
  const completedRaw = tasks.filter(t => isTaskDone(t)).sort(sortByPriority);
  const all = filterTasks(allRaw);
  const today = filterTasks(todayRaw);
  const scheduled = filterTasks(scheduledRaw);
  const anytime = filterTasks(anytimeRaw);
  const completed = filterTasks(completedRaw);

  const renderList = (list, elId) => {
    const el = document.getElementById(elId);
    if (!el) return;
    const addHtml = isAdding ? renderAddTask() : '';
    if (list.length === 0 && !isAdding) {
      el.innerHTML = `
        <div class="empty-state">
          <div class="empty-title">无任务</div>
          <div class="empty-desc">${hasActiveFilters() ? '没有符合筛选条件的任务' : '说 "TODOList xxx" 来添加'}</div>
        </div>
      `;
    } else {
      el.innerHTML = addHtml + list.map(renderTask).join('');
    }
  };

  renderList(all, 'allList');
  renderList(today, 'todayList');
  renderList(scheduled, 'scheduledList');
  renderList(anytime, 'anytimeList');
  renderList(completed, 'completedList');

  document.getElementById('allBadge').textContent = allRaw.length;
  document.getElementById('todayBadge').textContent = todayRaw.length;
  document.getElementById('scheduledBadge').textContent = scheduledRaw.length;
  document.getElementById('anytimeBadge').textContent = anytimeRaw.length;
  document.getElementById('completedBadge').textContent = completedRaw.length;
}

function startAdd() {
  isAdding = true;
  renderTasks();
  const input = document.getElementById('addInput');
  if (input) input.focus();
}

function saveAdd() {
  const input = document.getElementById('addInput');
  const prioritySelect = document.getElementById('addPriority');
  const deadlineInput = document.getElementById('addDeadline');
  const name = input ? input.value.trim() : '';
  const priority = prioritySelect ? prioritySelect.value : '中';
  const deadline = deadlineInput ? deadlineInput.value : null;
  if (name) {
    isAdding = false;
    addTask(name, priority, deadline);
  }
}

function cancelAdd() {
  isAdding = false;
  renderTasks();
}

function handleAddKey(event) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    saveAdd();
  } else if (event.key === 'Escape') {
    cancelAdd();
  }
}

function startEdit(id) {
  editingTaskId = id;
  renderTasks();
  const input = document.getElementById(`editInput-${id}`);
  if (input) { input.focus(); input.select(); }
}

function saveEdit(id) {
  const input = document.getElementById(`editInput-${id}`);
  const prioritySelect = document.getElementById(`editPriority-${id}`);
  const deadlineInput = document.getElementById(`editDeadline-${id}`);
  const linkInput = document.getElementById(`editLink-${id}`);
  const noteInput = document.getElementById(`editNote-${id}`);
  const newName = input ? input.value.trim() : '';
  const newPriority = prioritySelect ? prioritySelect.value : '中';
  const newDeadline = deadlineInput ? deadlineInput.value : null;
  const newLink = linkInput ? linkInput.value.trim() : '';
  const newNote = noteInput ? noteInput.value.trim() : '';
  if (newName) {
    updateTask(id, newName, newPriority, newDeadline, newLink, newNote);
  }
  editingTaskId = null;
  renderTasks();
}

function cancelEdit() {
  editingTaskId = null;
  renderTasks();
}

function handleEditKey(event, id) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    saveEdit(id);
  } else if (event.key === 'Escape') {
    cancelEdit();
  }
}

function switchTab(tab) {
  currentTab = tab;
  document.querySelectorAll('.tab').forEach(el => el.classList.remove('active'));
  document.querySelectorAll('.tab-panel').forEach(el => el.classList.remove('active'));
  const tabEl = document.querySelector(`.tab[data-tab="${tab}"]`);
  const panelEl = document.getElementById(`panel-${tab}`);
  if (tabEl) tabEl.classList.add('active');
  if (panelEl) panelEl.classList.add('active');
  renderTasks();
}

function escapeHtml(text) {
  if (!text) return '';
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#039;');
}

function normalizeUrl(url) {
  if (!url) return '';
  if (!/^https?:\/\//i.test(url)) return 'https://' + url;
  return url;
}

function showLoading() {
  const el = document.getElementById('loadingOverlay');
  if (el) el.classList.add('show');
}

function hideLoading() {
  const el = document.getElementById('loadingOverlay');
  if (el) el.classList.remove('show');
}

function showToast(message) {
  const el = document.getElementById('toast');
  if (!el) return;
  el.textContent = message;
  el.classList.add('show');
  setTimeout(() => el.classList.remove('show'), 2000);
}

async function toggleAlwaysOnTop() {
  isAlwaysOnTop = !isAlwaysOnTop;
  try {
    await invoke('set_always_on_top', { value: isAlwaysOnTop });
    const btn = document.getElementById('pinBtn');
    if (btn) btn.classList.toggle('active', isAlwaysOnTop);
  } catch (error) {
    console.error('Always on top error:', error);
  }
}

async function toggleCollapse() {
  isCollapsed = !isCollapsed;
  try {
    await invoke('toggle_collapse', { collapsed: isCollapsed });
  } catch (error) {
    console.error('Collapse error:', error);
  }
}

async function minimizeWindow() {
  try {
    await invoke('minimize_window');
  } catch (error) {
    console.error('Minimize error:', error);
  }
}

async function closeWindow() {
  try {
    await invoke('close_window');
  } catch (error) {
    console.error('Close error:', error);
  }
}

document.addEventListener('click', (e) => {
  const link = e.target.closest('.task-link');
  if (link) {
    e.preventDefault();
    const url = link.getAttribute('data-url');
    if (url) invoke('open_external', { url }).catch(() => {});
  }
});

window.addEventListener('DOMContentLoaded', () => {
  console.log('DOMContentLoaded');
  initVoice();
  renderTasks();
  loadTasks();
});
