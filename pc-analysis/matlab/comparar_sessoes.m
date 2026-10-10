clear;
clc;
close all;

%% 1. Importar os testes
proximo = readtable("lora_20261010_172641_082828.csv");
variavel = readtable("lora_20261010_173747_456161.csv");

sessoes = {proximo, variavel};
nomes = ["Proximo"; "Posicao variavel"];

tempos = cell(2,1);
dados = cell(2,1);
metricas = zeros(2,9);

%% 2. Calcular as metricas de cada sessao
for k = 1:2
    T = sessoes{k};

    % Converter o horario do CSV em datetime
    horario = datetime(string(T.pc_received_at_utc), ...
        'InputFormat', "uuuu-MM-dd'T'HH:mm:ss.SSSXXX", ...
        'TimeZone', 'UTC');

    % Tempo em segundos desde a primeira recepcao
    tempo = seconds(horario - horario(1));

    % Manter somente a primeira recepcao de cada sequencia
    [~, indices] = unique(T.sequence, 'stable');
    U = T(indices,:);

    dados{k} = U;
    tempos{k} = tempo(indices);

    % Valido para sessao sem reinicio ou volta da sequencia
    assert(all(diff(U.sequence) > 0), ...
        'Sequencia reiniciada ou fora de ordem: revisar a sessao.');

    esperados = max(U.sequence) - min(U.sequence) + 1;
    ausentes = esperados - height(U);
    recepcao = 100 * height(U) / esperados;

    metricas(k,:) = [ ...
        height(T), ...
        height(U), ...
        height(T) - height(U), ...
        ausentes, ...
        recepcao, ...
        mean(U.rssi_dbm), ...
        mean(U.snr_db), ...
        tempo(end), ...
        max(diff(tempo)) ...
    ];
end

%% 3. Exibir e salvar o resumo
resumo = array2table(metricas, 'VariableNames', { ...
    'Recepcoes', 'PacotesUnicos', 'Duplicatas', ...
    'SequenciasAusentes', 'RecepcaoIntervalo_pct', ...
    'RSSI_medio_dBm', 'SNR_medio_dB', ...
    'DuracaoObservada_s', 'MaiorIntervaloRecepcoes_s'});

resumo = addvars(resumo, nomes, ...
    'Before', 1, 'NewVariableNames', 'Sessao');

disp(resumo);
writetable(resumo, "resumo_20261010.csv");

%% 4. Comparar RSSI
figRSSI = figure('Color', 'w');
axRSSI = axes(figRSSI);

plot(axRSSI, tempos{1}, dados{1}.rssi_dbm, ...
    '.', 'MarkerSize', 10);
hold(axRSSI, 'on');
plot(axRSSI, tempos{2}, dados{2}.rssi_dbm, ...
    '.', 'MarkerSize', 10);

grid(axRSSI, 'on');
xlabel(axRSSI, 'Tempo desde a primeira recepcao (s)');
ylabel(axRSSI, 'RSSI (dBm)');
title(axRSSI, 'LoRa: comparacao de RSSI');
legend(axRSSI, nomes, 'Location', 'best');
drawnow;

exportgraphics(axRSSI, "comparacao_rssi_20261010.png", ...
    'Resolution', 200, 'BackgroundColor', 'white');

%% 5. Comparar SNR
figSNR = figure('Color', 'w');
axSNR = axes(figSNR);

plot(axSNR, tempos{1}, dados{1}.snr_db, ...
    '.', 'MarkerSize', 10);
hold(axSNR, 'on');
plot(axSNR, tempos{2}, dados{2}.snr_db, ...
    '.', 'MarkerSize', 10);

grid(axSNR, 'on');
xlabel(axSNR, 'Tempo desde a primeira recepcao (s)');
ylabel(axSNR, 'SNR (dB)');
title(axSNR, 'LoRa: comparacao de SNR');
legend(axSNR, nomes, 'Location', 'best');
drawnow;

exportgraphics(axSNR, "comparacao_snr_20261010.png", ...
    'Resolution', 200, 'BackgroundColor', 'white');