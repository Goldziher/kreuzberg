# frozen_string_literal: true

RSpec.describe Kreuzberg::CLIProxy do
  describe '.find_cli_binary' do
    context 'when binary exists' do
      it 'finds the binary in search paths' do
        binary = described_class.find_cli_binary
        expect(binary).to be_a(Pathname)
        expect(binary.file?).to be true
      end
    end

    context 'when binary does not exist' do
      before do
        allow(described_class).to receive(:search_paths).and_return([])
      end

      it 'raises MissingBinaryError' do
        expect do
          described_class.find_cli_binary
        end.to raise_error(Kreuzberg::CLIProxy::MissingBinaryError, /not found/)
      end
    end
  end

  describe '.call' do
    context 'when binary is available' do
      it 'executes CLI command successfully' do
        output = described_class.call(['--version'])
        expect(output).to be_a(String)
        expect(output).not_to be_empty
      end

      it 'raises CLIExecutionError on failure' do
        expect do
          described_class.call(['invalid-command'])
        end.to raise_error(Kreuzberg::CLIProxy::CLIExecutionError)
      end
    end
  end

  describe '.search_paths' do
    it 'returns an array of Pathname objects' do
      paths = described_class.search_paths('kreuzberg')
      expect(paths).to be_an(Array)
      expect(paths).to all(be_a(Pathname))
    end

    it 'includes expected search locations' do
      paths = described_class.search_paths('kreuzberg')
      path_strings = paths.map(&:to_s)

      expect(path_strings.any? { |p| p.include?('lib/bin') }).to be true
      expect(path_strings.any? { |p| p.include?('target/release') }).to be true
    end
  end

  describe '.root_path' do
    it 'returns a Pathname' do
      expect(described_class.root_path).to be_a(Pathname)
    end

    it 'points to an existing directory' do
      expect(described_class.root_path.directory?).to be true
    end
  end

  describe '.lib_path' do
    it 'returns a Pathname' do
      expect(described_class.lib_path).to be_a(Pathname)
    end

    it 'points to an existing directory' do
      expect(described_class.lib_path.directory?).to be true
    end
  end

  describe '.missing_binary_message' do
    it 'returns helpful error message' do
      message = described_class.missing_binary_message
      expect(message).to include('cargo build')
      expect(message).to include('kreuzberg-cli')
    end
  end
end
